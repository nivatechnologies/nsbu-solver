# Actual smooth pressure refinement family

P08 remains in progress. This increment independently constructs full doubled-band
physical mean-zero pressure from every actual smooth family branch, then compares
pressure and its complete gradient. All six states remain independent from rest.
See the [public guide](../../../docs/PRESSURE_REFINEMENTS.md).

[summary.json](summary.json), [source-sha256.json](source-sha256.json) and
[artifact-sha256.json](artifact-sha256.json) preserve results, 254 Rust plus 72
Python/stub source hashes, and the original/compressed hashes of raw reports.
The workspace Cargo manifest hash pins the optimized test profile explicitly.

## Required checks

- **350 Rust tests/probes pass**: 345 harness tests and five isolated allocator
  executables. No maintained test or source file is excluded.
- Executable lines: **21,992/22,236 (98.903%)**. Instrumented branches:
  **1,527/1,684 (90.677%)**. Both exceed the 80% requirement.
- Maxima: CC21, cognitive16, Halstead75.8956, physical-file469 and CRAP24.33594.
  Type-escape checking finds no matches. Every required gate passes.
- Formatting, strict Clippy, strict Rustdoc, fresh-target packaging, repository
  links/frozen hashes and the original mathematical runner pass. All 41 bootstrap
  tests pass; unchanged Python source retains its complete 188-test profile.
- A clean public export matches all 326 maintained source/stub files and the
  Cargo manifest. It runs the documented example, analytic pressure controls and
  actual six-branch pressure tests. The unchanged CLI retains prior clean-install
  evidence. Source-matched [Rust](hosted-rust.json) and
  [Python](hosted-python.json) hosted checks pass at commit `5d5378f`.

The first complete run passed all 350 tests but failed CRAP at **26.125** in
`PressureFamilyWorkspace::measure`. Independent force evaluation and cost
validation now have a separate function from report scheduling/publication.
The final complete test/coverage replay passes: those functions score **6** and
**9**, respectively. Initial raw coverage, metrics, CRAP and test output remain
preserved with the failure and correction described in the summary.

The test profile uses optimization level 2 with debug assertions and integer
overflow checks explicitly enabled. All targets and instrumented branches remain
in scope. Summed harness execution dropped from about 749 seconds in the preceding
unoptimized 345-test profile to 213 seconds in the first optimized 350-test run.
This is a local observation, not a portable speed guarantee or a change in
numerical qualification. Earlier unoptimized evidence remains historical.

## Numerical results and resource evidence

Independent Taylor–Green Fourier coefficients check full-band pressure outside
the original velocity grid, its mean-zero gauge, and known scalar/gradient RMS.
A force-only fixture preserves a mode beyond the finest velocity band. Refusal
and allocation tests cover full aggregate admission and spent failed attempts.

At time 1/512, the actual N=4/8/12 family gives pressure temporal RMS differences
**1.040466e-13** and **6.500614e-15**, and pressure-gradient differences
**1.132319e-12** and **7.074490e-14**. Both decrease by approximately 16.
CM/HO differences are **4.014818e-16** and **4.369252e-15**. Every state digest
remains unchanged during measurement. The known smooth analytical pressure zero
is never substituted for the integrated-state pressure.

The example reserves **28,614,160 bytes** for all simultaneous numerical owners,
below its 128 MiB cap. Pressure adds three attempts, **390 scalar transforms**,
**67,392 provider work units** and **34,977,792 weighted coefficient/sample visits**.
Its finite charges remain spent on failed calls. Reservations describe owned
storage and headers; allocator, caller output and process overhead are separate.

```sh
cargo test -p nsbu-benchmarks --lib pressure
cargo test -p nsbu-benchmarks --test pressure_family -- --nocapture
cargo test -p nsbu-benchmarks --test allocation
cargo run --release -p nsbu-benchmarks --example smooth_refinement
```

The complete pinned quality commands follow the [prior profile](../physical/README.md#commands),
using the recorded Cargo test settings and local `work/p08-pressure-family-`
paths. Informational duplication/dead-code findings and nonzero finding exits are
retained; mutation sweeps were not rerun. No dependency or reviewed input changed.

## Review and remaining work

The consumers share one private exact clock/policy/state binding. Independent
force/product assembly, reusable physical comparison, immutable reservations and
public reporting have separate responsibilities. Private complete results cannot
publish missing pair findings; diagnostic scratch never replaces accepted state.

Complete all-observable reference/force/arithmetic/sampling studies, full frozen
benchmark and accepted-artifact binding, concentrating global-mean/regional
pressure studies and window integration remain. Small spatial differences are
not supported error floors, and sampled maxima are not continuum enclosures.
**Accepted concentrating PDE windows: zero.**
