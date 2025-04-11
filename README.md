# Count Combinations

Optimized to rush.

Optimizations used:

- `with_capacity` to reduce allocs.
- `BufReader` to improve file reading to reduce waiting for the disk.
- `.par_bridge()` to use all CPU cores.
- `DashMap` to optimize concurrent write in a `HashMap` with the results.
- `with_capacity_and_shard_amount` to reduce allocs and concurrent access to keys in same shard.
- `lto = true` and `strip = true` to get a 614 KB to try to fit caches and better optimized machine code.
