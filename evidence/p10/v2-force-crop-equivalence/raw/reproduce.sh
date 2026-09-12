#!/usr/bin/env bash
set -euo pipefail
export PATH="/mnt/niva-array/nsbu-solver/work/rust-tools/bin:$PATH"
cargo fmt --all --check
cargo clippy -p nsbu-benchmarks --test v2_force_crop_equivalence --locked -- -D warnings -A dead_code
cargo test -p nsbu-benchmarks --test v2_force_crop_equivalence --locked -- --nocapture
RUSTDOCFLAGS='-D warnings' cargo doc -p nsbu-benchmarks --no-deps --locked
cargo +nightly-2026-03-03 llvm-cov clean --workspace
cargo +nightly-2026-03-03 llvm-cov --branch --no-report -p nsbu-benchmarks --test v2_force_crop_equivalence --locked -- --nocapture
cargo +nightly-2026-03-03 llvm-cov report --branch --json --output-path work/v2-force-crop-equivalence/coverage.json
rust-code-analysis-cli -p crates -m -O json > work/v2-force-crop-equivalence/metrics.jsonl
/mnt/niva-array/nsbu-solver/.venv/bin/python tools/check_repository.py
