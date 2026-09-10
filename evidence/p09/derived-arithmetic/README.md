# Complete smooth derived-field arithmetic

P08/P09 remain incomplete. This increment compares actual Rust diagnostic fields
with independent direct-DFT mathematics and independently evolved 80/120-digit
trajectories on N=4 and N=12, for both CM and HO, at time 1/512. It covers complete
velocity, gradient, Hessian, vorticity, physical pressure and pressure gradient.
The [public guide](../../../docs/DERIVED_ARITHMETIC.md) documents the full workflow.

[summary.json](summary.json) records all four executed studies and quality results.
[source-sha256.json](source-sha256.json) inventories every maintained source/stub;
[configuration-sha256.json](configuration-sha256.json) pins workflow/toolchain
inputs; [artifact-sha256.json](artifact-sha256.json) records compressed and original
sizes and hashes. Original reviewed mathematical artifacts are unchanged.

## Numerical evidence

Each study retains 46 scalar rows, including all ordered mixed Hessian entries,
on the full M=2N physical grid. Pressure includes the complete doubled quadratic
band and independently prescribed unprojected force, with global mean zero.
No analytical pressure or velocity is substituted for actual integrated data.

Six isolated precision profiles produce seven separate comparisons. The maximum
80/120-digit change relative to the corresponding Rust RMS/peak discrepancy is:

| Grid | Method | Maximum precision ratio | Elapsed study seconds |
| ---: | --- | ---: | ---: |
| 4 | CM | 7.052e-65 | 44.37 |
| 4 | HO | 1.375e-64 | 45.61 |
| 12 | CM | 5.188e-65 | 2103.70 |
| 12 | HO | 6.178e-65 | 2053.77 |

All four complete studies exit successfully and pass the declared ratio criterion
below 1e-40. Undefined zero denominators are not converted into supported floors.
The individual RMS/peak values, force effects, imaginary components and excluded
Nyquist-product defects are retained in the reports and summary.

For N=12 CM, same-state diagnostic RMS differences are approximately 1.305e-17
(velocity), 9.412e-17 (gradient), 6.046e-16 (Hessian), 9.465e-17 (vorticity),
2.127e-19 (pressure), and 6.416e-18 (pressure gradient). Relative to the independently
evolved 120-digit same-force trajectory, velocity/Hessian RMS differences are
1.722e-17 and 7.745e-16. These are sampled smooth-case arithmetic observations.
The error norms themselves are reduced in Python at 120 digits; this increment
does not separately measure the production Rust physical reducer's arithmetic.

All four actual Rust state payloads remain byte-identical to their previously
archived velocity exports. The existing public force and four trajectory inputs
per case are reused with explicit SHA-256 binding; all derived fields are newly
computed. Two separate processes run under a joint 64 GiB planning cap. Each N=12
study reserves 14,116,978,688 bytes under its 32 GiB cap, before reading its six
bounded artifacts. These are conservative planning allowances, not hard allocator
or rigorous roundoff guarantees.

## Complete quality and clean-source checks

- **355 Rust tests/allocation probes pass**: 350 harness tests and five isolated
  allocation executables across 258 Rust files.
- **198 Python tests pass**, including all reference, bootstrap and quality-tool
  tests. The complete inventory includes 82 Python/stub files.
- Rust executable lines: **22,290/22,551 (98.843%)**; instrumented branches:
  **1,549/1,712 (90.479%)**. Python executable lines: **4,765/4,775 (99.791%)**;
  branches: **948/958 (98.956%)**. No maintained scope is excluded.
- Rust maxima: CC21, cognitive16, Halstead75.8956, physical-file469,
  CRAP24.33594. Python maxima: CC16, cognitive19, Halstead13.8261,
  physical-file243, CRAP16. Every required gate passes.
- Strict typing reports zero errors/warnings; type-escape checks find no matches.
  Informational duplication/dead-code findings and their exit statuses are retained.
  Mutation sweeps were not rerun; historical reports do not measure the new code.
- Formatting, strict Clippy, strict Rustdoc, fresh-target public packaging,
  repository/frozen-input checks, all 41 bootstrap tests and the preserved
  mathematical runner pass.
- A clean public export matches all **340 maintained source/stub files**. Its
  release exporter reproduces all four complete Rust JSON artifacts byte-for-byte.
  Both exporter test targets, all ten new Python tests, and Python preflight pass
  from that export. Existing CLI installation evidence remains applicable.

The initial complete Rust run passed its tests but exposed CRAP72 in an untested
argument-processing `main`. The actual argument/export path now has a tested
`execute` function and a thin I/O wrapper. The complete source-matched replay
passes every gate; initial coverage, metrics, CRAP and test logs remain archived.
Source-matched [Rust](hosted-rust.json) and [Python](hosted-python.json) hosted
verification passes at commit `44e53f1`. Both preceding physical/pressure-family
increments also passed their Rust and Python hosted jobs.

## Reproduce and review

The public guide supplies artifact creation and comparison commands. The complete
quality commands follow the pinned repository workflows, using raw local evidence
paths prefixed `work/p09-derived-arithmetic-`. Coverage includes all workspace
targets and all Python tests, with the optimized Rust test profile retaining debug
assertions and overflow checks. The executed-source hash inventory also matches
the source snapshot taken before the long-running arithmetic studies began.

Evolution, diagnostic scratch, bit serialization, bounded artifact reading,
independent pressure/direct-DFT mathematics, precision scheduling and full tensor
reductions have separate responsibilities. Negative controls cover pressure
sign/gauge, fine-only force modes, every ordered tensor entry, malformed/nonfinite
payloads, changed force, explicit caps, undefined precision ratios and failed
partial reduction. Independent numerical implementations remain separate.

N=8 is supported by the workflow but is not rerun in this derived-field report.
Balances, residuals, regional/location observables, production reduction arithmetic,
complete artifact semantics and concentrating current-grid qualification remain.
**Accepted concentrating PDE windows: zero.**
