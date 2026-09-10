# Quality policy and evidence

Revision 2 · 10 September 2026

The active executable policy requires at least 80% executable-line coverage and
80% branch coverage, measured separately; CC and cognitive complexity below 22,
Halstead difficulty below 80, physical source/test files below 500 lines, CRAP
below 25 per function, and no Python `Any`/unresolved types or Rust `Any`/unknown
type escapes. CRAP is computed per function from actual branch outcomes as
`CC² × (1 − branch_coverage)³ + CC`. Mutation, duplication and dead-code tools
produce retained informational reports. Independent numerical oracles may have
intentional structural duplication and are reviewed without requiring a shared
implementation. Tool or numerical-test failures still fail CI.

The evidence below is historical: its full-coverage, zero-survivor and clean
review results remain useful measurements, but are no longer required gates.

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
| Executable line coverage | 3,144/3,144 | >=80% |
| Branch coverage | 558/558 | >=80% |
| CRAP, maximum per function | 15 | <25 |
| Non-equivalent surviving mutants | 0 | Informational |
| Confirmed dead code / duplicated blocks | 0 / 0 | Informational |
| Strict typing errors, including Any/unknown diagnostics | 0 | 0 |

Vulture's eight findings were reviewed individually: five discovered and executed
unittest classes, and three serialized TypedDict schema keys. Their exact findings
are retained beside the summary. Zero confirmed dead code and zero detected
duplicated blocks are scoped review/analysis results, not proofs about arbitrary
future use. The historical run has full branch coverage, so its CRAP values equal
cyclomatic complexity under the adopted formula.

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
workflow records the exact invocations and enforces per-function limits, 80%
line/branch coverage, file sizes and CRAP from measured branch outcomes.
Duplication, dead-code and mutation reports remain available for review.

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

CI reruns static analysis, coverage and numerical checks. It records mutation
evidence when available but does not require a zero-survivor result or a
source-hash replay on every push. The [initial baseline](../evidence/quality-baseline.json)
and earlier source snapshots remain historical evidence.

Rust push and pull-request checks keep the fast build, numerical tests, coverage,
metrics and CRAP gates mandatory. A complete `cargo-mutants` report is an optional
`workflow_dispatch` run (`full_mutation_report=true`); it must finish with a
structurally complete outcome report, while survivors and timeouts remain review
findings. Focused mutation work and the archived Python mutation reports are
historical evidence, including where their source scope predates quality tooling.

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


The [P09 lineage/image foundation](../evidence/p09/foundations/README.md) passes
202 tests/probes across 150 Rust files. All 10,275 executable lines, 850 instrumented
branches and 836 functions are covered. The complete 2,240-mutant run accounts for
2,079 caught and 161 unviable, with zero survivors/timeouts. Maxima remain CC21,
cognitive15, Halstead74.4231, physical-file336 and CRAP21. Strict lint, clone detection
and scoped public-API/SOLID review remain clean. Dedicated allocation instrumentation
covers image capture/refusal, restoration and registry operations. Identity, ancestry
and physical payload ownership remain separate; no component can confer PDE
acceptance. Hosted checks and the full P09 package remain incomplete.


The [recorded-step/controller/balance increment](../evidence/p09/recorded/README.md)
passes 221 tests/probes across 168 Rust files. All 11,612 executable lines, 942
instrumented branches and 926 functions are covered. The full 2,353-mutant run
caught 2,171 and found 182 unviable, with zero survivors/timeouts. Maxima are CC21,
cognitive16, function/file Halstead74.4231, physical-file336 and CRAP21. Strict
Clippy, clone detection and scoped review have zero dead/redundant-code findings
or dynamic type escapes. Four allocation executables now include repeated complete
recorded commits with no allocation/reallocation/free inside the attempt loop.

SOLID review separates controller decisions, read-only observation, compensated
balance arithmetic, bounded log storage and exclusive transaction ownership.
Private corruption and actual CM/HO fixtures cover rollback and exact admission
boundaries. Four long scientific studies run in full coverage and are skipped
only during mutation reruns. No production or equivalent-mutant exemptions are
used. Full checkpoints and hosted verification of this increment remain pending.

The [P09 artifact/replay increment](../evidence/p09/artifacts/README.md) records
226 tests/probes, 11,879 executable lines and 956 instrumented branches in its
full local replay, with 2,374 mutation outcomes retained. It adds artifact and
replay coverage but does not complete P09.

The [owned smooth runtime increment](../evidence/p09/runtime/README.md) records
267 tests/probes across 198 Rust files, 99.20% executable line coverage and 95.02%
branch coverage. Maxima are CC21, cognitive16, Halstead75.8956, physical-file400
and CRAP21. Format, strict Clippy, Rustdoc, package/install and installed CLI checks
pass. Its 17 clone findings are reviewed and informational; mutation sweeps were
not rerun. Ownership, observation, resource admission and binary I/O remain
separate responsibilities. Imported bytes cannot create trusted provenance.

The revised-policy Python checker CLI contract fix also passed
[source-matched hosted CI](../evidence/quality-policy-v2/cli-contract/hosted-ci.json),
including all 163 tests and automated coverage/CRAP gates. The owned-runtime Rust
increment also passed source-matched hosted verification; P08/P09 and concentrating qualification
remain incomplete.

The [accepted-reconstruction/file-I/O increment](../evidence/p09/reconstruction/README.md)
passes 284 local tests/probes across 209 Rust files. Coverage is 99.08% executable
lines and 91.56% branches; maxima are CC21, cognitive16, Halstead75.8956,
physical-file385 and CRAP24.33594. The final combined run covers the library,
CLI, tests and runnable example. Measured CRAP failures in earlier drafts led to
separate admission, I/O and snapshot-validation responsibilities and deterministic
reader regressions. The 27 duplication findings are informational and reviewed;
no new mutation sweep was requested. Local package, clean-source install and 14
installed CLI checks pass. Source-matched hosted Rust and Python verification also passes.

The [reconstruction archive increment](../evidence/p09/reconstruction-archive/README.md)
passes 292 local tests/probes across 215 Rust files, with 99.08% executable line
coverage and 91.44% branch coverage. Maxima remain CC21, cognitive16,
Halstead75.8956, physical-file385 and CRAP24.33594. Additional earlier-node
metadata regressions cover the externally supplied history contract. Clippy,
Rustdoc and fresh-target packaging pass. Its 31 duplication findings are
informational; no mutation sweep was rerun. Source-matched hosted Rust and Python checks pass.

The [independent experiment and replay increment](../evidence/p09/experiments/README.md)
passes 305 local tests/probes across 226 Rust files. Coverage is 99.01% executable
lines and 91.23% instrumented branches; maximum CC21, cognitive16, Halstead75.8956,
physical-file385 and CRAP24.33594 pass the required limits. Separate joint
admission tests cover inadequate caps and overflow. Raw reports preserve the
corrected example CRAP finding and final measurements. Duplication reports 32
informational matches; no mutation sweep was rerun. Format, Clippy, Rustdoc, fresh
packaging, clean-source example/install and installed CLI checks pass. Source-matched
hosted Rust and Python checks also pass.

The [same-grid arithmetic increment](../evidence/p09/arithmetic/README.md) passes
313 Rust tests/probes and 184 Python tests. Rust coverage is 98.92% of executable
lines and 91.24% of instrumented branches; Python coverage is 99.79% and 98.99%.
Rust maxima are CC21, cognitive16, Halstead75.8956, file385 and CRAP24.33594.
Python maxima are CC16, cognitive19, Halstead13.8261, file243 and CRAP16; strict
typing reports zero errors/warnings. Source inventories cover 229 Rust and 70
Python/stub files. Splitting admitted evolution from state serialization resolves
an exporter CRAP34.125 finding. Raw reports retain the finding and final evidence.
The nine Python dead-code findings identify discovered tests or schema fields;
duplication is reported and no mutation sweep was rerun. Full source/export/input
identity, cap failures and high-mode/reality negative controls are tested.
Source-matched hosted Rust and Python verification pass; no PDE window is accepted.

The [canonical protocol increment](../evidence/p08/protocol/README.md) passes
321 Rust tests/probes across 234 source/test files. Coverage is 98.93% executable
lines and 91.29% branches; maxima remain CC21, cognitive16, Halstead75.8956,
file385 and CRAP24.33594. Format, Clippy, Rustdoc and fresh packaging pass.
The unchanged Python source retains its preceding complete quality evidence;
41 bootstrap tests and frozen mathematical checks were rerun. Informational
Clippy and duplication output is preserved, including nonzero finding exits.
Borrowed settings, canonical encoding and numerical review remain separate from
physical provenance. Source-matched hosted Rust and Python checks pass; no concentrating window is accepted.

The [physical derivative increment](../evidence/p08/derivatives/README.md) passes
332 Rust tests/probes across 241 Rust files and 188 Python tests across the
72-file Python/stub inventory. Rust coverage is 98.92% lines and 91.07% branches;
Python coverage is 99.79% lines and 99.01% branches. All required complexity,
Halstead, file-length, CRAP and typing gates pass. Generic tensor collectors
preserve vector aliases and full ordered derivative multiplicities; scalar
sampling, regional geometry and reference evaluation remain separate. Clean-source
fixture regeneration, public tests and installation pass. Informational findings
and the coverage-combination recovery are recorded without hiding nonzero tool
exits. Source-matched hosted Rust and Python verification pass; no concentrating window is accepted.

The [complete physical comparison increment](../evidence/p08/physical/README.md)
passes 341 Rust tests/probes across 247 Rust files. Coverage is 98.93% executable
lines and 91.05% branches; maxima remain CC21, cognitive16, Halstead75.8956,
file385 and CRAP24.33594. All 72 Python/stub files match the preceding verified
188-test source profile. Forty-one bootstrap tests were rerun; fresh public
source tests, packaging and frozen mathematical checks pass. Independent field
comparison, tensor reduction and regional geometry remain separate, and actual
CM/HO state digests are preserved. Informational finding exits remain explicit.
Source-matched hosted checks pass. P08 and concentrating qualification remain incomplete.

The [actual physical refinement family](../evidence/p08/physical-family/README.md)
passes 345 Rust tests/probes across 250 Rust files: 98.93% executable lines and
90.72% instrumented branches. Maxima CC21, cognitive16, Halstead75.8956,
physical-file426 and CRAP24.33594 meet all gates. All 72 Python/stub files retain
the verified 188-test profile; 41 bootstrap tests were rerun. Complete schedule
admission, exact-policy/state binding and borrowed numerical sampling have
separate responsibilities. Cap and failed-attempt controls pass with no
post-construction allocation. A matching clean export runs the documented
example and focused checks. Source-matched hosted checks pass; P08 remains incomplete.

The test profile now optimizes numerical loops at level 2 while explicitly
retaining debug assertions and integer overflow checks. All workspace targets,
negative controls, isolated allocation executables and line/branch instrumentation
remain in scope. This profile is pinned in the workspace Cargo manifest and
recorded with the pressure increment's evidence. Earlier unoptimized profiles
remain historical evidence; optimization does not turn any numerical finding
into a qualified PDE result.

The [actual pressure family](../evidence/p08/pressure-family/README.md) passes
350 Rust tests/probes across 254 Rust files: 98.90% executable line coverage and
90.68% instrumented branch coverage. Maxima CC21, cognitive16, Halstead75.8956,
physical-file469 and CRAP24.33594 pass all gates. The initial pressure report
function's CRAP26.125 finding is preserved. Separating prescribed-force assembly
from complete report publication resolves it, and a complete replay verifies the
final source. Clean-source tests/example, fresh packaging and frozen bootstrap
checks pass; unchanged Python retains its 188-test profile. Source-matched hosted checks pass. No concentrating PDE window is accepted.

The [derived-field arithmetic increment](../evidence/p09/derived-arithmetic/README.md)
passes 355 Rust tests/probes and 198 Python tests across 258 Rust and 82 Python/stub
files. Rust line/branch coverage is 98.84%/90.48%; Python is 99.79%/98.96%.
Maxima remain within every required gate: Rust CC21/cognitive16/Halstead75.8956/
LOC469/CRAP24.33594; Python CC16/cognitive19/Halstead13.8261/LOC243/CRAP16.
The initial untested CLI-wrapper CRAP72 finding and the complete corrected replay
are preserved. Clean-source exports match all four actual artifacts, all four
N=4/N=12 derived-field studies pass, and bootstrap/frozen-input checks pass.
Hosted checks for this increment are pending. No concentrating window is accepted.
