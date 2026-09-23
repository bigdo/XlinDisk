//! Benchmark harness.
//!
//! Exists from the first PR because "the same capability keeps getting faster"
//! is part of the point of the core. The actual workloads land in PR8 together
//! with the release gate; what is recorded here is the list, so nobody has to
//! reinvent it later.

fn main() {
    println!("xlindisk-bench (skeleton)");
    for suite in [
        "traversal: wide-tree, deep-tree, many-small-files, few-large-files, mixed",
        "grouping: 1M synthetic entries, 10M synthetic entries",
        "duplicate: zero, sparse, dense, large-files, many-small-files",
    ] {
        println!("  {suite}");
    }
    println!("metrics: wall time, entries/sec, MB/s, bytes read, peak RSS, CPU, time-to-first-result, bytes per retained entry");
    println!("PR8 adds: duplicate intermediate peak memory, 1/2/8 worker determinism, change-during-scan races");
}
