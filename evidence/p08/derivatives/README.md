# Physical derivative and tensor-error increment

P08 remains in progress. This increment adds full-band scalar Fourier
sampling through second physical derivatives, complete scalar/vector/tensor error
accumulation, v2 regional tensor reports and independently checked analytical
reference derivatives. [The public guide](../../../docs/DERIVATIVE_DIAGNOSTICS.md)
documents coordinate, pressure-gauge, normalization and allocation contracts.

[summary.json](summary.json) records the execution profile and actual results.
[source-sha256.json](source-sha256.json) identifies all **241 Rust and 72
Python/stub files**. [artifact-sha256.json](artifact-sha256.json) records exact
compressed and original raw-report hashes.

## Results

- **332 Rust tests/probes** pass: 327 harness tests and five isolated allocation
  executables. Line coverage is **20,328/20,550 (98.920%)**; branch coverage is
  **1,459/1,602 (91.074%)**.
- **188 Python tests** pass. Coverage is **4,276/4,285 executable lines (99.790%)**
  and **802/810 branches (99.012%)**. The subprocess-patched run produced parallel
  data files; after an initial premature JSON export reported no data, combining
  those files and exporting the complete report succeeded without repeating tests.
- Rust maxima: CC21, cognitive16, Halstead75.8956, file385, CRAP24.33594. Python:
  CC16, cognitive19, Halstead13.8261, file243, CRAP16. Strict Python typing reports
  zero errors/warnings/notes. Required gates pass.
- Format, strict Clippy with informational dead-code reporting, strict Rustdoc,
  fresh-target public packaging, frozen repository/math checks and clean-source
  installation pass. The clean export matches all 313 maintained source/stub files
  and reproduces the documented derivative tests and reference output. Its installed
  CLI help and HO smooth preflight succeed.
- Nine v2 point cases retain all 46 derivative/reference entries. The largest
  componentwise scaled Rust/120-digit error is **3.4583e-14**; the largest observed
  80/120-digit change is **3.2174e-74**. Selected derivatives also agree with
  independent scalar numerical differentiation in Python.
- Actual independently integrated N=4 CM/HO trajectories reach `1/512` in eight
  committed full/two-half proposals. Maximum sampled Hessian errors are about
  **9.648e-13 (CM)** and **2.279e-13 (HO)**. Value and gradient errors are also
  retained. Sampling leaves the actual Fourier states unchanged.

These are pointwise and small smooth diagnostic results. They do not establish a
concentrating PDE trajectory, continuum maximum, pressure mean quadrature or
complete current-grid reference/arithmetic channel. **Accepted PDE windows: zero.**

## Reproduction

Use the pinned toolchain and documented Python quality environment. Numerical
fixture and focused commands appear in the public guide. The complete runs used:

```sh
cargo fmt --all -- --check
cargo clippy --workspace --all-targets --locked -- -D warnings -A dead_code
RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked
cargo +nightly-2026-03-03 llvm-cov --workspace --all-targets --branch \
  --no-default-ignore-filename-regex \
  --ignore-filename-regex '(/rustc/|/\.cargo/registry/|/\.rustup/toolchains/|/target/)' \
  --json --output-path work/rust-coverage.json
rust-code-analysis-cli -p crates -m -O json > work/rust-metrics.json
python quality/check_crap.py rust work/rust-metrics.json work/rust-coverage.json work/rust-crap.json
python -m coverage run -m pytest quality/tests reference/tests tools/tests -q
python -m coverage combine
python -m coverage json
radon cc -s -j quality reference tools > work/python-cc.json
radon hal -j quality reference tools > work/python-halstead.json
complexipy quality reference tools --max-complexity-allowed 21 --color no
basedpyright
python quality/check_crap.py python work/python-cc.json work/full-coverage.json work/python-crap.json
cargo package --workspace --locked --allow-dirty --target-dir <fresh-target>
python tools/check_repository.py
python tools/verify_design.py --output work/design-checks.json
```

Actual retained local report paths use `work/p08-derivative-` prefixes. The Rust
reference comparison log additionally uses `work/p08-reference-derivatives-tests.log`.
The clean-source export ran the public focused commands, regenerated identical
reference JSON, installed the CLI and exercised `--help` and
`smooth --dry-run --method ho`. All 41 bootstrap tests ran in the complete Python
suite. No dependency version or preserved reviewed input changed. Source-matched [hosted Rust](hosted-rust.json) and [Python](hosted-python.json) verification pass.

## Review

Spectral differentiation, fixed-storage statistics, geometry and analytical
reference evaluation have separate responsibilities. The existing vector APIs
remain aliases of the more general tensor collectors. Ordered tensor entries
preserve Frobenius multiplicity; scalar pressure does not inherit a vector
normalization. Allocation probes cover workspace admission/refusal/reuse and
pointwise reference plus regional tensor accumulation.

The 41 Rust duplication matches include existing numerical/test structures and
small independent analytic fixtures. The one Python match is existing malformed
clock-fixture setup. Ten Vulture findings are discovered test classes or JSON
schema declarations. Separate nonzero informational tool exits and raw reports
are retained; strict Clippy passes with the documented dead-code allowance.
No mutation sweep was rerun under the revised informational policy.
