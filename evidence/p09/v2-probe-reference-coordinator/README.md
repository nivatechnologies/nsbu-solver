# Reconstructed reference coordinator evidence

Source commit: `6634c35dbb7048754c1d9cc59953e047c335c816`
Source tree: `0d35049fb7d8735f5dca9ee96d2f38057622ff43`

Focused routing:

```text
cargo test -p nsbu-benchmarks --test v2_diagnostic_coordinator publishes_crossed_consumers_in_one_unqualified_manifest --locked -- --exact --nocapture
PASS: 1 passed; 147.37 s
```

This compares the coordinator's reconstructed-reference report with a separately owned `ProbeReferenceWorkspace` for every one of the seven actual manifest publications. It checks the common publication clock/identity/origins, all six source domains, layout/floors, branch slots, quantity tags and all 24 `LocalError` values.

Allocation:

```text
cargo test -p nsbu-benchmarks --test v2_diagnostic_coordinator_allocation --locked -- --nocapture
PASS: admission allocations 0; construction 64,014,112 bytes <= joint bound 74,716,312; seven-event steady execution allocations/deallocations/reallocations 0.
```

Static checks:

```text
cargo clippy -p nsbu-benchmarks --lib --locked -- -D warnings
cargo clippy -p nsbu-benchmarks --test v2_diagnostic_coordinator --test v2_diagnostic_export --test v2_diagnostic_coordinator_allocation --locked -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked
```

All passed. Whole `crates` rust-code-analysis reports maximum function CC 21, cognitive 21, maximum all-node Halstead difficulty 75.9295154185022, and no Rust file at or above 500 physical lines. Raw JSON Lines output is preserved as `metrics-6634.json.gz`.

The full numerical export test and focused LLVM coverage/CRAP were not rerun locally. The export targets compile, and the cheap pressure/residual-grid profile test passes. Whole-maintained 80% line/branch coverage remains a hosted integration gate; this evidence does not claim it.
