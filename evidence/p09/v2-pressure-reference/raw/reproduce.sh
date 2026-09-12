#!/usr/bin/env bash
set -euo pipefail
export PATH="/mnt/niva-array/nsbu-solver/work/rust-tools/bin:$PATH"
cargo fmt --all -- --check
cargo test -p nsbu-benchmarks --test v2_pressure_reference --test v2_pressure_reference_allocation --locked -- --nocapture
cargo test -p nsbu-benchmarks --lib v2_experiment::pressure --locked -- --nocapture
cargo clippy -p nsbu-benchmarks --lib --test v2_pressure_reference --test v2_pressure_reference_allocation --locked -- -D warnings -A dead_code
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked
cargo +nightly-2026-03-03 llvm-cov clean --workspace
cargo +nightly-2026-03-03 llvm-cov --branch --no-report -p nsbu-benchmarks --test v2_pressure_reference --locked -- --nocapture
cargo +nightly-2026-03-03 llvm-cov --branch --no-report -p nsbu-benchmarks --test v2_pressure_reference_allocation --locked -- --nocapture
cargo +nightly-2026-03-03 llvm-cov report --branch --json --output-path work/v2-pressure-reference-coverage.json
rust-code-analysis-cli -p crates -m -O json > work/v2-pressure-reference-metrics.json
