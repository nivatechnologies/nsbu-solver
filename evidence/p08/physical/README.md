# Complete physical-field comparison increment

P08 remains in progress. This increment assembles complete scalar, velocity,
gradient, Hessian and vorticity comparisons on one unaligned sample grid. It
retains modes found only on the finer source, preserves supplied pressure means
and feeds the same complete samples into the v2 regional partition. See the
[public guide](../../../docs/PHYSICAL_COMPARISONS.md) for APIs and numerical limits.

[summary.json](summary.json) records the execution profile and actual results;
[source-sha256.json](source-sha256.json) covers **247 Rust and 72 Python/stub
files**. [artifact-sha256.json](artifact-sha256.json) records compressed and original
hashes for every preserved raw report.

## Verification

- **341 Rust tests/probes pass:** 336 harness tests and five isolated allocation
  executables. Coverage is **20,926/21,152 executable lines (98.932%)** and
  **1,486/1,632 branches (91.054%)**.
- Maximum CC21, cognitive16, Halstead75.8956, physical-file385 and CRAP24.33594
  pass every required threshold. No Rust `Any`/`unknown` type escapes are found.
- Formatting, strict Clippy with informational dead-code reporting, strict
  Rustdoc and fresh-target public packaging pass. Repository/frozen-input checks,
  all **41 bootstrap tests**, and the original mathematical runner pass.
- A fresh public source export matches all **319 maintained source/stub files**
  and passes the documented physical-comparison, tensor, regional and actual
  trajectory tests. The preceding derivative increment supplies clean CLI
  installation evidence; CLI source has not changed in this increment.
- All 72 Python/stub source files match the preceding derivative increment.
  Its complete **188-test** quality profile and source-matched hosted checks are
  reused explicitly; bootstrap tests were rerun for this repository increment.

Actual independently evolved smooth CM/HO states at `1/256` give sampled RMS
method differences of approximately **7.706e-12 for velocity**, **6.848e-11 for
its gradient**, **6.085e-10 for its Hessian**, and **6.848e-11 for vorticity**.
Their complete Fourier-state digests remain unchanged during measurement.
These are N=4 smooth diagnostics, not concentrating or continuous-time qualification.
**Accepted PDE windows: zero.**

## Commands

Commands ran from the source worktree using the pinned toolchains and tools:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings -A dead_code
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked
cargo test -p nsbu-solver --test physical_comparison --test tensor_errors
cargo test -p nsbu-benchmarks --test regional_physical --test integrated_physical -- --nocapture
cargo +nightly-2026-03-03 llvm-cov --workspace --all-targets --branch \
  --no-default-ignore-filename-regex \
  --ignore-filename-regex '(/rustc/|/\.cargo/registry/|/\.rustup/toolchains/|/target/)' \
  --json --output-path work/rust-coverage.json
rust-code-analysis-cli -p crates -m -O json > work/rust-metrics.json
python quality/check_crap.py rust work/rust-metrics.json work/rust-coverage.json work/rust-crap.json
cargo package --workspace --locked --allow-dirty --target-dir <fresh-target>
python tools/check_repository.py
python -m unittest discover -s tools/tests -v
python tools/verify_design.py --output work/design-checks.json
```

Actual raw paths use the local `work/p08-physical-` prefix. Informational
Clippy/duplication exits are preserved separately from passing required checks;
shared discovered-test helpers and independent analytic fixtures are reviewed
as intended uses. Mutation sweeps were not rerun under the revised policy.
No dependency or frozen reviewed input changed. Hosted verification is pending.

## Review and remaining work

Derivative schedules, scalar FFT sampling, tensor reductions and geometric
partitions have separate responsibilities. Two scalar samplers and four reusable
physical arrays avoid storing all tensor entries per point. Allocation probes
cover aggregate refusal, complete comparisons and regional reduction. Malformed
later components cannot expose partial reports, and a later valid call resets
scratch. Norm-of-difference tests reject the false zero obtained by subtracting
the magnitudes of opposite vectors; signed curl and fine-only-mode fixtures
check separate operator contracts.

The comparison view records geometry and quantity, but the owning experiment
must bind the mathematical identity, exact time and actual-state provenance.
It must also budget its finite measurement schedule, including failures. Complete
benchmark inventory binding, all-channel refinement production and concentrating
qualification remain required by the active plan.
