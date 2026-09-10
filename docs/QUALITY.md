# Quality evidence

The Python implementation, including P00C diagnostic code, passes numerical and
quality checks locally and in hosted CI. P00B, P00C and Rust foundations are
complete. Concentrating PDE qualification remains separate from these checks.

The [source-hashed measurement report](../evidence/quality-reference/summary.json)
records 155 passing tests. The scope includes maintained Python implementations,
reference oracles, tooling and tests. Typing stubs are inventoried and type checked;
they have no executable coverage obligations. Frozen review files, generated
fixtures, archived execution provenance and third-party dependencies are separately
inventoried. Declarative workflow/configuration files are reviewed and hashed,
without being counted as Python functions or executable Python lines.

| Metric | Measured result | Required |
|---|---:|---:|
| Cyclomatic complexity, maximum per function | 15 | <22 |
| Cognitive complexity, maximum per function | 17 | <22 |
| Halstead difficulty, maximum per file / function | 9.888 / 7.805 | <80 |
| Physical lines per source/test file, maximum | 243 | <500 |
| Executable line coverage | 3,144/3,144 | 100% |
| Branch coverage | 558/558 | 100% |
| CRAP, maximum per function | 15 | <25 |
| Non-equivalent surviving mutants | 0 | 0 |
| Confirmed dead code / duplicated blocks | 0 / 0 | 0 / 0 |
| Strict typing errors, including Any/unknown diagnostics | 0 | 0 |

Vulture's eight findings were reviewed individually: five discovered and executed
unittest classes, and three serialized TypedDict schema keys. Their exact findings
are retained beside the summary and checked for drift in CI. Zero confirmed dead
code and zero detected duplicated blocks are scoped review/analysis results, not
proofs about arbitrary future use. Every measured function has full branch coverage;
CRAP therefore equals its cyclomatic complexity under the adopted formula.

## Mutation evidence

The [reconciled mutation report](../evidence/quality-reference/mutations/summary.json)
contains 8,691 current mutation specifications: 8,603 explicit test failures,
86 individually documented equivalents, zero non-equivalent survivors, zero
timeouts and zero untested cases. Two Cosmic Ray `ExceptionReplacer` construction
errors on dotted exception names are invalid; neither is counted as a kill.

Each equivalent retains its exact diff and justification. Some equivalences rely
on the documented entry points and CPython 3.12 public-only execution profile;
others follow from integer-domain constraints, overwritten buffers or identities
in the truncated jet algebra. The HO coefficient remainder equivalence includes
an integral-kernel argument and 6,476 arithmetic comparisons. Equivalents are never
silently relabeled as killed mutants.

The [raw runs](../evidence/quality-reference/mutations/runs.json) preserve complete
inventories, source hashes, commands and outcomes. The original comprehensive run
contained 7,909 specifications; removing redundant wrapper defaults required a new
inventory for those modules. P00C adds 786 specifications without changing any
P00B implementation target. A fresh inventory matches all 8,691 consolidated
specifications and their current implementation hashes. The new runs and test-source
snapshots are archived beside the earlier reports.

Cosmic Ray labels process timeouts as `killed`. An earlier supplemental jet report
incorrectly included one such timeout among its kills. That historical report is
corrected and retained; the comprehensive reconciliation resolved the timeout with
an explicit failure. Initial focused timeout reruns did not collect the complete
suite; the later runs used explicit complete file lists. Import/work probes now
run before full collection, so excessive numerical work produces an assertion
failure rather than consuming the mutation-process timeout.

## Reproduction and CI

Install `requirements-quality.txt` in the configured `.venv`. Run `coverage run -m
pytest -q`, `coverage combine`, `coverage json`, `basedpyright`, Radon's `cc` and
`hal` commands, `complexipy`, `vulture` and Pylint's `duplicate-code` check. The
workflow records the exact invocations and enforces per-function limits, exact
line/branch coverage, file sizes, duplication and reviewed dead-code findings.
With full branch coverage, the CC gate also enforces CRAP <25.

Mutation execution must use a disposable checkout because the local distributor
changes target files in place. Activate its environment, then run:

```sh
cosmic-ray init .cosmic-ray.toml work/mutations.sqlite
cosmic-ray baseline .cosmic-ray.toml
cosmic-ray exec .cosmic-ray.toml work/mutations.sqlite
cosmic-ray dump work/mutations.sqlite > work/mutation-results.jsonl
```

This serial reproduction can be lengthy. Preserve raw outcomes and review each
survivor; the configuration does not hide equivalents or convert timeouts into
project-level kills. Archived parallel runs include their precise executed commands.

CI reruns static analysis, coverage and numerical checks. Its mutation-evidence
gate checks the complete reconciled report against exact source/configuration
hashes and refuses added or changed source files until their evidence is renewed.
It reuses matching mutation evidence; it does not claim to rerun every mutation
on every push. The [initial baseline](../evidence/quality-baseline.json) and earlier
source snapshots remain historical evidence.

## Responsibility review

Scalar differentiation and implicit jets remain independent, as do coefficient
convolution and grid products. CM and HO coefficient construction remain separate.
Tuple-shape checks, JSON narrowing, field evaluation, quadrature, DFT operations,
time stepping and report I/O have distinct responsibilities. Integrators receive
an RHS callable, never a reference-state assignment interface. Argparse now owns
each wrapper default in one place. Import tests prohibit numerical work during
library loading; bounded-work tests cover root, coefficient and quadrature limits.

Report doubles verify orchestration and policy only. Mathematical claims rely on
separately executed high-precision studies, and those studies do not establish a
qualified concentrating trajectory or a production Rust implementation.

## Rust workspace profile

P01 now has separately measured Rust quality evidence. See the
[per-metric report](../evidence/p01/README.md) and
[pinned tool declarations](../quality/rust/README.md). These measurements cover
all five current Rust source/test files; no numerical implementation is implied.
The Rust workflow reruns mutation testing, in addition to coverage and static
checks. Production builds use stable Rust; instrumented branch coverage uses a
separately pinned nightly toolchain and explicitly includes integration tests.

P02 extends the measured Rust scope to numerical foundations and their public
contract tests. [Its evidence](../evidence/p02/README.md) records full line/branch
coverage, bounded complexity, zero surviving mutants and allocation/clock failure
tests. This supersedes P01's source-scope measurements while preserving that
package's historical evidence.

P03 extends the Rust profile to spectral operators. [Its evidence](../evidence/p03/README.md)
records direct-sum and convolution checks, allocation instrumentation, full quality
measurements and the hosted verification status. Earlier package reports remain
historical snapshots, not measurements of the expanded current source.

P04 adds an independently checked CM kernel, provider budgets, exact interval
admission and transactional attempts. [The package evidence](../evidence/p04/README.md)
records its expanded measurements and final verification state. Both allocation
probe executables and all helper/test source remain in the measured scope.

P05 adds the independent exact-v2 benchmark implementation and provider. Its
[quality and numerical evidence](../evidence/p05/README.md) includes every current
Rust source/test file, fixed-degree algebra, resource admission and allocation
instrumentation. Cancellation-limited components remain explicitly unqualified.

P06 extends the Rust profile to bounded trajectory scheduling and independent
smooth/concentrating comparisons. [Its complete evidence](../evidence/p06/README.md)
records 105 tests/probes, 5,319 executable lines and 480 instrumented branches
covered, with 1,444 mutations accounted for and no survivors or timeouts. The
package passes locally and in hosted CI; concentrating PDE qualification remains
unresolved.

P07 adds the independently constructed HO method and expanded scalar/PDE
comparisons. [Its source-matched evidence](../evidence/p07/README.md) records all
120 tests/probes, complete line/branch coverage and 1,537 accounted-for mutants
with zero survivors or timeouts. All local and source-matched hosted gates pass,
completing P07. The concentrating diagnostic still does not qualify a PDE window.


The first [P08 diagnostics increment](../evidence/p08/core/README.md) passes all
local core checks: 145 tests/probes; 7,545 executable lines, 594 instrumented
branches and 614 functions covered; 1,802 mutations with 1,677 caught, 125 unviable,
and zero survivors/timeouts. Maximum cyclomatic complexity is 21, cognitive
complexity 15, Halstead difficulty 74.4231, physical file length 264, and CRAP 21.
Clone detection and strict lint/review find zero redundant/dead-code findings and
no `Any`/unknown types. Raw LLVM region/instantiation statistics remain visible;
they are not the executable-line/instrumented-branch gate.

The SOLID review keeps force evaluation independent of state, diagnostics dependent
on borrowed field/clock contracts, and reconstruction separate from integration
and commit. Conservative products independently verify the rotational path;
shared FFT/geometry primitives retain their earlier independent tests. A common
smooth test runner removes duplicated resource/transaction setup. Allocation
instrumentation verifies the new conservative workspace's reservation and zero
allocation during evaluation. Three long scientific studies run in complete
validation/coverage and are skipped only during focused mutation reruns; no
production mutation source is excluded. Source-matched hosted Rust and Python
checks passed; this diagnostics increment does not complete P08.

The later [P08 reporting and balance increment](../evidence/p08/reporting/README.md)
passes its local gates: 171 tests/probes, 8,779 executable lines, 696 instrumented
branches and 703 functions fully covered across 128 Rust files. The full mutation
run accounts for 2,034 mutants: 1,900 caught, 134 unviable, zero survivors/timeouts.
CC21, cognitive15, Halstead74.4231, physical-file264 and CRAP21 remain below the
requested limits. Clone detection, strict linting and scoped review remain clean.

Sampling, geometric classification, error accumulation and state integration have
separate responsibilities. The regional collector depends on narrow immutable
clock/geometry contracts and preserves global errors alongside every local mask.
Failed classification attempts remain charged to a finite work allowance. Shared
mode traversal and diagnostic force assertions remove actual duplication.

The mutation run's production source matches the final snapshot byte-for-byte.
Its test-source hashes and executed profile are retained; the additive balance
study and shared history-test helper followed that snapshot. All four long
scientific studies run in complete coverage; the final mutation profile skips
only those long reruns and excludes no production source. Source-matched hosted
Rust and Python verification passed the final snapshot. P08's complete window verifier is still
pending, and no concentrating PDE window is qualified.


The [bounded window measurement increment](../evidence/p08/window/README.md)
passes 195 tests/probes with 9,733 executable lines, 810 instrumented branches and
792 functions fully covered across 142 Rust files. All 2,175 mutations are
accounted for: 2,023 caught, 152 unviable, zero survivors or timeouts. Maximum CC21,
cognitive15, Halstead74.4231, physical-file336 and CRAP21 satisfy the requested
limits. Clone detection, strict Clippy and scoped public-API/SOLID review find no
dead/redundant code or dynamic type escapes. CI now checks per-function CC and
cognitive totals including nested closures, matching the local measurements.

Exact time geometry, evidence rules, budget accounting and streaming review retain
separate responsibilities. The numerical reviewer borrows immutable policies and
cannot modify integrated state or issue a PDE acceptance label. Full-field and
missing-evidence negative controls pass. Four long scientific studies run in full
coverage and are skipped only during mutation reruns; no production exclusion or
equivalent-mutant exemption is used. Raw reports preserve all other LLVM metrics.
Hosted verification remains pending for this increment.
