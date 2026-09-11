# Reference oracle complexity correction

The local whole-workspace RCA run on source revision
`678d3dd046e84d46600771c3707b250a05455f53` exposed a whole-file RCA unit
Halstead difficulty of 96.2409 for
`crates/nsbu-benchmarks/tests/v2_reference_oracle.rs`. The earlier oracle
summary reported function metrics only and therefore missed that unit-level
value. Hosted Rust run `34596983665` for source revision
`441da25d79cf51439c5b3e99b43c9828de1ba2db` failed separately on the regional
test's cognitive complexity.

The oracle is now split into sibling `core`, `tracking`, `legacy`, and existing
`phase` modules. The signed Fourier reconstruction, legacy comparison,
thresholds, fixtures, and assertions are unchanged. Whole-node RCA now reports
maximum unit Halstead difficulty 74.5987, maximum function cognitive complexity
21, maximum function cyclomatic complexity 10, and maximum module physical
lines 152. The largest function cognitive complexity is 21, below the 22
threshold.

Final focused validation on the correction worktree:

```text
cargo test -p nsbu-benchmarks --test v2_reference_tracking -- --nocapture  => 4 passed, 0 failed
cargo fmt --all -- --check                                                   => passed
cargo clippy -p nsbu-benchmarks --test v2_reference_tracking --locked -- -D warnings => passed
```

The oracle remains an independent legacy comparison and does not establish PDE
qualification, convergence, or a finite-time singularity claim.
