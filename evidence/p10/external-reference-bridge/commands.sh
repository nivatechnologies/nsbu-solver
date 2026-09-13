#!/usr/bin/env bash
set -euo pipefail

quality_dir=$(mktemp -d)
trap 'rm -rf "$quality_dir"' EXIT

cargo fmt --check --manifest-path evidence/p10/external-reference-bridge/harness/Cargo.toml
cargo test --manifest-path evidence/p10/external-reference-bridge/harness/Cargo.toml
cargo clippy --manifest-path evidence/p10/external-reference-bridge/harness/Cargo.toml --all-targets -- -D warnings
rust-code-analysis-cli -m -O json -p evidence/p10/external-reference-bridge/harness/src > "$quality_dir/metrics.jsonl"
grep -v 'bridge/bridge_tests.rs' "$quality_dir/metrics.jsonl" > "$quality_dir/metrics-production.jsonl"
cargo +nightly-2026-03-03 llvm-cov --manifest-path evidence/p10/external-reference-bridge/harness/Cargo.toml --all-targets --branch --json --output-path "$quality_dir/coverage.json"
python3 evidence/p10/snapshot-comparison-adapter/sanitize_coverage.py "$quality_dir/coverage.json"
python3 quality/check_crap.py rust "$quality_dir/metrics-production.jsonl" "$quality_dir/coverage.json" "$quality_dir/crap-production.json"
