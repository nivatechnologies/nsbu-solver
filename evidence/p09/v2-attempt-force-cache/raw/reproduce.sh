#!/usr/bin/env bash
set -euo pipefail
export PATH="/mnt/niva-array/nsbu-solver/work/rust-tools/bin:$PATH"
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings -A dead_code
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked
cargo test -p nsbu-benchmarks --lib runtime_force::attempt_cache::tests --locked -- --nocapture
cargo test -p nsbu-benchmarks --test runtime_force --test v2_attempt_force_cache --test v2_attempt_force_cache_allocation --test v2_archive --locked -- --nocapture
cargo test -p nsbu-solver --test force_budget --locked
# Run cargo-llvm-cov clean, then the same solver/benchmark selections with --branch --no-report,
# followed by: cargo +nightly-2026-03-03 llvm-cov report --branch --json.
rust-code-analysis-cli -p crates -m -O json > work/v2-attempt-force-cache-metrics.jsonl
