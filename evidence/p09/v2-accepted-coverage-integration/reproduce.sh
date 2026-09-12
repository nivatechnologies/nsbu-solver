#!/usr/bin/env bash
set -euo pipefail
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings -A dead_code
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked
python3 tools/check_repository.py
cargo test -p nsbu-benchmarks --test v2_region_coverage --locked
cargo test -p nsbu-benchmarks --test v2_diagnostic_coordinator --locked
cargo test -p nsbu-benchmarks --test v2_diagnostic_export --locked
cargo test -p nsbu-benchmarks --test v2_diagnostic_coordinator_allocation --locked
