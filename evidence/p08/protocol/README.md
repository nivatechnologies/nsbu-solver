# Canonical numerical protocol increment

P08 remains in progress. This increment binds every admitted numerical rule,
ordered observable key, exact tested clock and off-stage reconstruction geometry
to a reproducible SHA-256 identity. It supplies a canonical byte artifact and a
bounded numerical review using those same settings. It cannot authenticate
benchmark semantics or physical-state provenance and accepts no PDE window.

The [public format guide](../../../docs/PROTOCOL_FORMAT.md) describes the API,
byte format, independent golden fixture, resource scope and refusal behavior.
[summary.json](summary.json) and [source-sha256.json](source-sha256.json) identify
the measured source. [artifact-sha256.json](artifact-sha256.json) records compressed
and original hashes for the retained raw reports.

## Executed checks

- Complete Rust LLVM branch-coverage run: **316 harness tests and five isolated
  allocation executables pass**. Coverage is **19,770/19,984 executable lines
  (98.929%)** and **1,426/1,562 branches (91.293%)**.
- Maximum CC21, cognitive16, Halstead75.8956, physical file385 and CRAP24.33594
  pass their required limits across **234 maintained Rust source/test files**.
- Format, strict Clippy with informational dead-code reporting, strict Rustdoc,
  and fresh-target packaging of all three public crates pass.
- Repository/frozen-input checks, all **41 bootstrap guard tests**, and the
  original mathematical runner pass. Fresh results match its preserved report.
- All **70 Python/stub files** match the previously verified arithmetic source
  inventory byte-for-byte. Its full 184-test quality profile is reused explicitly;
  the bootstrap tests above were rerun for these documentation/package changes.
- No Rust `Any`/`unknown` type escapes are found. The 34 duplication findings
  remain informational. A separate Clippy run without the dead-code allowance
  reports shared test/example helpers unused in particular compilation targets;
  its nonzero exit is retained, not counted as a lint-gate pass. Mutation sweeps
  were not rerun under the revised informational policy.

Commands ran from the source worktree with Rust 1.94.0, RCA 0.0.25,
cargo-llvm-cov 0.9.1 and the existing Python 3.12 virtual environment:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings -A dead_code
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked
cargo test -p nsbu-solver --test protocol --test protocol_allocation
cargo +nightly-2026-03-03 llvm-cov --workspace --all-targets --branch \
  --no-default-ignore-filename-regex \
  --ignore-filename-regex '(/rustc/|/\.cargo/registry/|/\.rustup/toolchains/|/target/)' \
  --json --output-path work/rust-coverage.json
rust-code-analysis-cli -p crates -m -O json > work/rust-metrics.json
python quality/check_crap.py rust work/rust-metrics.json \
  work/rust-coverage.json work/rust-crap.json
cargo package --workspace --locked --allow-dirty --target-dir <fresh-target>
python tools/check_repository.py
python -m unittest discover -s tools/tests -v
python tools/verify_design.py --output work/design-checks.json
```

The actual local output paths use the `work/p08-protocol-` prefix; raw reports are
preserved here in gzip form. Packaging used a fresh temporary registry/target to
avoid stale same-version dependencies. No dependency version or frozen reviewed
input changed. Hosted verification of this source increment is pending.

## Review and limits

Settings, canonical serialization, numerical review and physical provenance have
separate responsibilities. The protocol borrows validated arrays and has no
trajectory mutation interface. Hashing and writing share one canonical traversal;
the fixture's expected hash was independently encoded in Python. Short output
buffers are refused before modification, trailing bytes remain unchanged, and
post-admission operations allocate nothing.

The generic component requires nonzero problem/semantics identifiers but does
not validate their meaning. Complete benchmark inventory binding, all-observable
experiment production, current-grid comparisons and accepted-state lineage remain
required by the active plan. Protocol verification does not qualify a PDE trajectory.
