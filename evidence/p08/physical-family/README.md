# Actual smooth physical refinement family

P08 remains in progress. This increment binds complete velocity, gradient,
Hessian and vorticity comparisons to six actual independent rest trajectories,
exact accepted clocks, immutable numerical policy words and finite observation
budgets. See the [public guide](../../../docs/PHYSICAL_REFINEMENTS.md).

[summary.json](summary.json) records the execution profile and results.
[source-sha256.json](source-sha256.json) covers 250 Rust and 72 Python/stub files;
[artifact-sha256.json](artifact-sha256.json) preserves compressed/original report
hashes. The unchanged Python inventory reuses its complete 188-test derivative
profile explicitly; all 41 bootstrap tests were rerun.

## Actual verification

- **345 Rust tests/probes pass**: 340 harness tests and five isolated allocation
  executables, including the executable public walkthrough.
- Executable line coverage is **21,426/21,657 (98.933%)**; instrumented branch
  coverage is **1,515/1,670 (90.719%)**, over the complete maintained Rust scope.
- Maxima are CC21, cognitive16, Halstead75.8956, physical-file426 and
  CRAP24.33594. All required gates pass; no type escapes are found.
- Format, strict Clippy, strict Rustdoc and fresh-target public packaging pass.
  Informational dead-code/duplication findings and their exits are preserved;
  no mutation sweep was rerun under the revised policy.
- A clean public source export matches all 322 maintained source/stub files.
  It runs the updated release example and focused physical/family tests. CLI
  source is unchanged; prior clean installation evidence remains applicable.
- Repository links/frozen hashes, the original mathematical runner and 41
  bootstrap tests pass. Source-matched hosted checks are pending for this
  increment. The preceding physical comparison source passed both hosted jobs.

The example independently evolves N=4/8/12, CM/HO, with steps 64/32/16 at quantum
2^-16 to tick 128 (time 1/512). Its complete joint reservation is **25,407,872
bytes**, below the 128 MiB cap. Three physical observations charge **1,350 scalar
inverse FFTs** and **91,111,872 weighted coefficient/sample visits**. The latter
excludes FFT internals and is not a FLOP count.

At the endpoint, temporal RMS differences decrease by approximately 16 for all
four quantities. CM/HO RMS differences are approximately 1.602e-14 (velocity),
1.424e-13 (gradient), 1.265e-12 (Hessian) and 1.424e-13 (vorticity). Complete state
digests remain unchanged. Small spatial differences and exact rest zeros do not
establish supported error floors or concentrating convergence.

## Reproduction and review

```sh
cargo test -p nsbu-benchmarks --test physical_family -- --nocapture
cargo test -p nsbu-benchmarks --test allocation
cargo test -p nsbu-solver --test physical_comparison --test allocation
cargo run --release -p nsbu-benchmarks --example smooth_refinement
```

Complete coverage used the pinned nightly, cargo-llvm-cov branch instrumentation,
all workspace targets and the prior unoptimized test profile. RCA includes every
maintained source/test file; CRAP joins function complexity to instrumented
coverage. Full gate commands follow the [preceding profile](../physical/README.md#commands),
with raw local paths using the `work/p08-physical-family-` prefix.

Immutable planning, actual-state binding, reusable sampling and output formatting
have separate responsibilities. Private report construction requires all twenty
quantity/pair results; shared input borrows protect integrated states. Cap/overflow,
invalid floors/grids, stale clocks, changed policy/manifest, terminated-family and
spent-attempt failures have explicit controls. Rebinding an admitted source pair
cannot grow diagnostic storage or change the constructor's default domains.

Complete pressure/reference/force/arithmetic/sampling evidence, full benchmark
observable/protocol/artifact binding and concentrating integration remain.
**Accepted concentrating PDE windows: zero.**
