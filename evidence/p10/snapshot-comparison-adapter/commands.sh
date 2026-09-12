#!/usr/bin/env bash
set -euo pipefail

cargo fmt --manifest-path evidence/p10/snapshot-comparison-adapter/harness/Cargo.toml -- --check
cargo test --manifest-path evidence/p10/snapshot-comparison-adapter/harness/Cargo.toml
cargo clippy --manifest-path evidence/p10/snapshot-comparison-adapter/harness/Cargo.toml --all-targets -- -D warnings
rust-code-analysis-cli -m -O json -p evidence/p10/snapshot-comparison-adapter/harness/src > evidence/p10/snapshot-comparison-adapter/raw/metrics.jsonl
grep -v 'harness/src/tests.rs' evidence/p10/snapshot-comparison-adapter/raw/metrics.jsonl > evidence/p10/snapshot-comparison-adapter/raw/metrics-production.jsonl
cargo +nightly-2026-03-03 llvm-cov --manifest-path evidence/p10/snapshot-comparison-adapter/harness/Cargo.toml --all-targets --branch --json --output-path evidence/p10/snapshot-comparison-adapter/raw/coverage.json
python3 evidence/p10/snapshot-comparison-adapter/sanitize_coverage.py evidence/p10/snapshot-comparison-adapter/raw/coverage.json
python3 quality/check_crap.py rust evidence/p10/snapshot-comparison-adapter/raw/metrics-production.jsonl evidence/p10/snapshot-comparison-adapter/raw/coverage.json evidence/p10/snapshot-comparison-adapter/raw/crap-production.json
