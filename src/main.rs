use std::fs::File;
use std::io;
use dicgen::DictionaryGenerator;
use std::io::{BufRead, BufReader, Write};
use std::io::BufWriter;
use std::path::{Path, PathBuf};
use rayon::prelude::*;
use dashmap::{DashMap, DashSet};
use serde::Serialize;
use clap::Parser;

const BUFFER_SIZE: usize = 1024 * 1024 * 4;
const EXPECTED_COMBINATIONS: usize = 20_000;
const SHARDS: usize = 128;

#[derive(Parser)]
struct Opt {
    #[clap(short, long, default_value = "3")]
    combination_length: usize,
    #[clap(short, long, default_value = "output.txt")]
    output: PathBuf,
    #[clap(short, long)]
    #[arg(index = 1)]
    input: PathBuf,
    #[clap(short, long)]
    alphabet: Option<String>,
    /// Include combinations with zero occurrences.
    #[clap(long, default_value = "false")]
    include_zeros: bool,
}

fn main() -> std::io::Result<()> {
    let time = std::time::Instant::now();
    let opt = Opt::parse();

    let alphabet_string = opt.alphabet
        .or_else(|| alphabet_of(&opt.input).ok())
        .ok_or_else(|| std::io::ErrorKind::InvalidInput)?;

    let mut combinations = Vec::with_capacity(EXPECTED_COMBINATIONS);
    for combination in dict_for_with_len(alphabet_string, opt.combination_length)? {
        combinations.push(combination);
    }

    let counts = count_combinations(&opt.input, &combinations)?;
    save_counts(opt.output, counts, opt.include_zeros)?;
    
    println!("Done in {:?}", time.elapsed());
    Ok(())
}

fn alphabet_of<P: AsRef<Path>>(path: P) -> io::Result<String> {
    let alphabet = DashSet::<char>::with_capacity(27);

    BufReader::with_capacity(BUFFER_SIZE, File::open(path)?)
        .lines()
        .filter_map(|line| line.ok())
        .par_bridge()
        .for_each(|line| {
            line.par_chars().for_each(|c| {
                if !alphabet.contains(&c) {
                    alphabet.insert(c);
                }
            });
        });

    Ok(
        alphabet
            .into_iter()
            .collect::<String>()
    )
}

fn dict_for_with_len(alphabet: String, len: usize) -> io::Result<DictionaryGenerator> {
    assert!(alphabet.len() > 1, "Alphabet string is empty");
    let first = alphabet.chars().next().ok_or_else(|| {
        io::Error::new(
            io::ErrorKind::InvalidInput,
            "Alphabet string is empty",
        )
    })?;
    let mut start = String::with_capacity(len);
    for _ in 0..len {
        start.push(first);
    }

    let last = alphabet.chars().last().unwrap();
    let mut end = String::with_capacity(len);
    for _ in 0..len {
        end.push(last);
    }

    DictionaryGenerator::new(alphabet, start, end)
        .map_err(|error| {
            io::Error::new(
                io::ErrorKind::InvalidInput,
                format!("Invalid input format string: {}", error),
            )
        })
}

fn count_combinations<P: AsRef<Path>>(path: P, combinations: &Vec<String>) -> io::Result<DashMap<&str, usize>> {
    let counts = DashMap::<&str, usize>::with_capacity_and_shard_amount(EXPECTED_COMBINATIONS, SHARDS);
    for combination in combinations.iter() {
        counts.insert(combination, 0);
    }

    BufReader::with_capacity(BUFFER_SIZE, File::open(path)?)
        .lines()
        .par_bridge()
        .filter_map(|line| line.ok())
        .filter(|line| line.len() >= 3)
        .for_each(|line| {
            combinations
                .par_iter()
                .filter(|&combination| line.contains(combination))
                .for_each(|combination| {
                    let mut count = counts.get_mut(combination.as_str()).unwrap();
                    *count += 1;
                });
        });

    Ok(counts)
}

#[derive(Serialize)]
struct CombinationCount<'a> {
    key: &'a str,
    count: usize,
}

fn save_counts<P: AsRef<Path>>(path: P, counts: DashMap<&str, usize>, include_zeros: bool) -> io::Result<()> {
    let mut writer = BufWriter::new(File::create(path)?);

    for (key, count) in counts {
        if count > 0 || include_zeros {
            let combination_count = CombinationCount { key, count };
            serde_json::to_writer(&mut writer, &combination_count)?;
            writer.write(b",\n")?;
        }
    }
    writer.flush()?;

    Ok(())
}
