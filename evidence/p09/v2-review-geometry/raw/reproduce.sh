cargo fmt --all -- --check
cargo test -p nsbu-benchmarks --test v2_review_profile --locked
cargo clippy -p nsbu-benchmarks --lib --test v2_review_profile --locked -- -D warnings -A dead_code
RUSTDOCFLAGS='-D warnings' cargo doc -p nsbu-benchmarks --no-deps --locked
cargo +nightly-2026-03-03 llvm-cov clean --workspace
cargo +nightly-2026-03-03 llvm-cov --branch --no-report -p nsbu-benchmarks --test v2_review_profile --locked
cargo +nightly-2026-03-03 llvm-cov report --branch --json --output-path work/review-geometry-evidence/coverage.json
rust-code-analysis-cli -p crates -m -O json > work/review-geometry-evidence/metrics.jsonl
python quality/check_crap.py rust work/review-geometry-evidence/review-profile-metrics.jsonl work/review-geometry-evidence/coverage.json work/review-geometry-evidence/review-profile-crap.json
