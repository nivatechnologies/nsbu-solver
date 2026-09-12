# Reproducing the off-stage pressure evidence

The numerical and coverage measurements bind to source commit `d687b585d324699a484e15a91581e41977392007`. Commit `5ec1975e056ec84fce402b60f7cd6b2cee1eccb7` changes only the oracle-scope prose. `source-sha256.json` lists every changed measurement-source file and its byte digest.

Run from the repository root with the locked dependency graph:

```sh
cargo test -p nsbu-benchmarks --test v2_probe_pressure --locked -- --test-threads=1 --nocapture
cargo test -p nsbu-benchmarks --test v2_probe_pressure_allocation --locked -- --test-threads=1
cargo clippy --workspace --all-targets --locked -- -D warnings -A dead_code
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked
PATH="/mnt/niva-array/nsbu-solver/work/rust-tools/bin:$PATH" cargo +nightly-2026-03-03 llvm-cov -p nsbu-benchmarks --test v2_probe_pressure --test v2_probe_pressure_allocation --branch --no-default-ignore-filename-regex --ignore-filename-regex '(/rustc/|/\.cargo/registry/|/\.rustup/toolchains/|/target/)' --json --output-path coverage.json -- --test-threads=1 --nocapture
python3 quality/check_crap.py rust changed-metrics.jsonl coverage.json crap.json
```

The archived `d687-coverage.log.gz` contains the instrumented numerical and allocator output. The 66.73-second normal test result remains in the execution transcript; no separate raw file was captured. `final-all-target-clippy.log.gz` was captured at evidence commit `3ec6e5e`, whose Rust source and tests are byte-identical to `d687`.

The reported 77.7778% branch rate is the focused changed-file subset. It is below the local 80% target and is not presented as a global gate result. Combined hosted CI owns the global coverage decision.
