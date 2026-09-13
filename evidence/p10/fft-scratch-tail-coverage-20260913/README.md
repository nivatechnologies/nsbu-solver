# Scratch-tail FFT changed-file coverage

Exact source `9eba11f196a25f0843f0cbd0f4ed08c9f7ae4645` was measured with the repository-pinned nightly and cargo-llvm-cov 0.9.1. Only ordinary tests for package `nsbu-solver` ran, serially, with two build jobs. All 251 tests passed; no ignored tests or benchmark/CLI packages ran.

Across the five changed production FFT files, line coverage is 425/449 (94.65%) and branch coverage is 33/40 (82.50%). The tiled traversal files `transform.rs` and `transform/tiled.rs` each have 100% line and branch coverage. Per-file gaps below 80% are recorded without exclusions in `changed-files-summary.json`: `fft.rs` has 33.33% line coverage, `avx.rs` has 75% branch coverage, and `workspace.rs` has 77.27% branch coverage. `fft.rs` has no instrumented branches.

These numbers cover the five changed production files as whole files. They do not claim 80% coverage for the whole project. Raw LLVM JSON, annotated text output, commands, versions, test output, timings, and hashes are preserved here.
