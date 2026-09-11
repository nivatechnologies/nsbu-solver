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
Source-matched hosted checks pass. No concentrating window is accepted.

The [sampling-family increment](../evidence/p08/sampling-family/README.md) passes
362 Rust tests/probes across 265 Rust files, with 98.82% executable line and
90.24% branch coverage. Maxima CC21, cognitive18, Halstead75.8956, physical-file469
and CRAP24.33594 pass every gate. The unchanged 82-file Python inventory retains
the verified 198-test profile. A complete initial 357-test profile is preserved,
followed by a full replay after integrating the verified derived-field increment.
The clean source reproduces every example finding; frozen/bootstrap and package
checks pass. Source-matched hosted Rust and Python checks pass; no concentrating window is accepted.


The [streaming probe increment](../evidence/p08/probe-family/README.md) passes
367 Rust tests/probes across 272 Rust files: 98.81% executable line and 90.11%
branch coverage. Maxima CC21, cognitive18, Halstead75.8956, physical-file471 and
CRAP24.33594 pass all gates. The unchanged 82-file Python/stub inventory retains
its verified 198-test profile; 41 bootstrap tests were rerun. Clean-source tests
and complete example reproduction, packaging and frozen mathematical checks
pass. Admission, scheduling, interpolation and publication remain separate, with
no reference-assignment interface. Source-matched hosted Rust and Python checks pass; no
concentrating PDE window is accepted.


The [global reference-gauge increment](../evidence/p09/reference-gauge/README.md)
passes 209 Python tests across 88 Python/stub files: 99.80% executable line and
98.89% branch coverage. Maxima CC16, cognitive19, Halstead13.8261, physical-file243
and CRAP16 meet all gates; strict typing has zero errors. All 272 Rust files are
unchanged from the verified 367-test/probe source. A clean export reproduces the
public study, and all nine numerical studies plus frozen/bootstrap checks pass.
The less accurate initial pilot and its deliberately interrupted partial test
run remain recorded. The final complete coverage run passes. Source-matched hosted Rust and Python checks pass; no concentrating PDE window is accepted.


The [reconstructed physical-field increment](../evidence/p08/probe-diagnostics/README.md)
passes 372 Rust tests/probes across 281 Rust files: 98.79% executable line and
90.25% branch coverage. Maxima CC21, cognitive18, Halstead75.8956, physical-file473
and CRAP24.33594 pass every gate. All 88 Python/stub files match the verified
209-test reference profile. Complete clean-source output and eleven Python
regressions pass after integrating the Python-only increment without altering
measured Rust source, Cargo settings or runtime fixtures. Bootstrap/frozen checks,
strict linting, Rustdoc and fresh packaging pass. Hosted Python passes; hosted Rust numerical checks pass but the prose-only lexical scan fails, repaired in the following increment;
accepted concentrating PDE windows remain zero.


## P08 streamed residual progress

The [streamed-residual increment](../evidence/p08/streamed-residuals/README.md) passes 378 Rust tests/probes
across 288 Rust files: 98.75% executable line and 90.29% branch coverage. Maxima
CC21, cognitive18, Halstead75.8956, physical-file475 and CRAP24.33594 meet every
gate. All 88 Python/stub files match the verified 209-test profile. Clean-source
replay reproduces thirty branch defects and twenty-five complete coefficient-field
comparisons; strict linting, Rustdoc, fresh packaging and frozen/bootstrap checks
pass. Five exact non-stage probes retain original accepted-node histories and
strict nested temporal geometry. Refusal, stale-origin, terminal later-child and
allocation controls pass. Source-matched hosted Rust and Python checks pass.

The preceding hosted failure is preserved: a lexical type scan rejected English
prose in a full documentation comment after numerical checks passed. CI now
retains all lexical matches and excludes full comment lines from the refusal
classification. Actual prohibited code identifiers remain rejected in explicit
negative controls. Numerical source and measured coverage are unchanged by this
workflow repair.

P08/P09 remain incomplete: current-grid arithmetic, complete reference/force,
balance/quadrature, regional/location, benchmark/artifact and concentrating
qualification remain. No concentrating PDE window is accepted.


## P09 production physical-reduction arithmetic progress

The [reduction-arithmetic increment](../evidence/p09/reduction-arithmetic/README.md) passes 385 Rust tests/probes
and 220 Python tests across 294 Rust and 98 Python/stub files. Rust line/branch
coverage is 98.69%/90.11%; Python is 99.82%/98.82%. Maxima remain Rust
CC21/cognitive18/Halstead75.8956/LOC475/CRAP24.33594 and Python
CC16/cognitive19/Halstead13.8261/LOC243/CRAP16. Strict typing, linting, Rustdoc,
fresh packaging and frozen/bootstrap checks pass. Source-matched hosted checks
of this reduction-arithmetic increment and the preceding streamed residuals pass.

Two actual N=4/N=12 smooth CM/HO component datasets independently verify both
production tensor-reduction entry paths. All 144 per-statistic comparisons meet
the declared 80/120 precision separation. Magnitude construction and accumulation
of already-rounded magnitudes remain separately measured. The clean source
reproduces both complete packet pairs and all numerical findings byte-for-byte.
No FFT or trajectory arithmetic is relabeled by these reduction measurements.

P08/P09 remain incomplete. Residual/balance/regional/location arithmetic, full
reference/force/transfer refinements, benchmark/artifact binding and concentrating
current-grid qualification remain. No concentrating PDE window is accepted.

## P08 independently reconstructed balance quadrature progress

The [balance/quadrature increment](../evidence/p08/balance-probes/README.md) passes
391 Rust tests/probes across 302 Rust files. Executable line/branch coverage is
98.68%/89.93%; maxima CC21, cognitive18,
Halstead75.8955, physical-file477 and CRAP24.33594
meet all gates. All 98 Python/stub files match the verified 220-test reference
profile. Strict linting, Rustdoc, fresh packaging, 41 bootstrap tests and frozen
mathematical checks pass. Hosted verification of this increment is pending.

The six independent smooth histories supply sixty complete conservative balance
samples at ten exact physical times and eighteen Simpson integrals on three
strictly refined schedules. Quadrature changes are measured without altering
integration steps or reconstructed fields. All six child-failure positions and a
failure after staged quadrature updates preserve work and prevent partial reports.
Clean-source replay reproduces the complete resource and numerical output exactly.
Two original CRAP failures are retained. The final consumer removes a tautological
clock/origin comparison after proving both views borrow one immutable private
record; complete owner/manifest binding and all six real failure controls remain.
Fresh complete coverage and quality gates pass after the simplification.

P08/P09 remain incomplete. Complete residual/balance arithmetic, reference/force,
regional/location and transfer refinements, benchmark/artifact binding and current
grid concentrating qualification remain. No concentrating PDE window is accepted.

## P09 exact force-evaluation optimization progress

The [force-evaluation increment](../evidence/p09/force-evaluation/README.md) passes
398 Rust tests/probes across 306 Rust files. Executable line/branch coverage is
98.68%/89.99%; maxima CC21, cognitive18,
Halstead75.8955, physical-file477 and CRAP24.33594
meet all gates. All 98 Python/stub files match the verified 220-test profile;
41 bootstrap tests and frozen mathematical checks rerun. Strict linting, Rustdoc,
fresh packaging and complete clean-source profile reproduction pass. Hosted checks
for this increment are pending.

Axial root/jet entries are rebuilt at every exact-clock request, and immutable
jet derivative indices replace repeated searches. The original point evaluator,
independent derivative oracle and high-precision force fixture remain separate.
All tested force/derivative words and full force spectra match. Three interleaved
baseline/combined passes retain all 36 coefficient hashes with median measured
speed ratio 3.046 on N8/N12/N16 profiles. Allocation-free requests and prior
failure contracts pass. These small-grid timings are not large-grid estimates.

P08/P09 remain incomplete. This profile evolves no PDE trajectory. Current-grid
arithmetic, force/reference/transfer refinements and complete all-observable
provenance integration remain; accepted concentrating windows remain zero.

## P09 persistent parallel force-sampling progress

The [parallel-force increment](../evidence/p09/parallel-force/README.md) passes
405 Rust tests/probes across 315 Rust files. Executable line/branch coverage is
98.67%/89.86%; maxima CC21, cognitive18,
Halstead75.8955, physical-file477 and CRAP24.33594
meet all gates. The unchanged 98 Python/stub files retain their complete 220-test
profile. Strict linting, Rustdoc, fresh packaging, 41 bootstrap tests and frozen
mathematical checks pass. Hosted checks for this increment are pending.

Persistent workers own disjoint axial planes with preflighted buffers and
configured stack allowances. Original pointwise arithmetic and serial FFTs
preserve all tested coefficient words and reported work. Every submitted worker
is collected before returning, including structured error and panic controls;
failed pools terminate without publishing incomplete force spectra. The first,
repeated, nonmonotone and refused requests allocate nothing after construction.

A clean source export reproduces all 413 maintained source hashes, six focused
tests and the isolated allocation executable. N16/N32 preflight and complete
serial/parallel numerical reports reproduce after excluding elapsed seconds only.
The measured N32 profile is about 12.44 times faster at 32 workers than the already
optimized serial baseline; these shared-host force-only timings do not estimate
large-grid integration or establish mathematical force accuracy.

P08/P09 remain incomplete. The concentrating workflow still needs complete
current-grid arithmetic, force/reference/transfer refinements and all-observable
provenance binding. No concentrating PDE window is accepted.


## Runtime alpha · 11 September 2026

[Runtime alpha evidence](../evidence/runtime-alpha/README.md) records all local
R01–R04 exits on the complete 332-file Rust inventory: 425 harness tests plus
nine isolated allocation executables (434 tests/probes), and one compiled public
Rustdoc example. Coverage includes maintained tests and numerical examples;
only toolchain, registry and build-output paths are excluded.

| Metric | Measured | Required |
|---|---:|---:|
| Executable lines | 28,822/29,242 (98.56%) | ≥80% |
| Instrumented branches | 1,989/2,240 (88.79%) | ≥80% |
| Maximum cyclomatic complexity | 21 | <22 |
| Maximum cognitive complexity | 18 | <22 |
| Maximum function/file Halstead difficulty | 75.8956 | <80 |
| Maximum physical lines per maintained Rust file | 477 | <500 |
| Maximum per-function CRAP | 24.33594 | <25 |
| Type escape identifiers in the declared Rust scan | 0 | 0 |

All 98 maintained Python/stub files match the retained 220-test profile by
SHA-256; 41 bootstrap tests and the original mathematical verification were
rerun. This is explicit source-bound reuse, not a claim that the complete Python
suite was rerun locally for this increment. Hosted CI reruns that suite for the
release commit. Strict Clippy, formatting, Rustdoc, clean workspace packaging,
fresh CLI installation, default CM/HO runs and checkpoint/resume all pass.

The SOLID review separates admission, runtime ownership, force selection,
measurement arithmetic, checkpoint encoding/admission, and CLI parsing,
execution, reporting and file publication. Serial/parallel force selection
implements the existing provider contract; independent numerical oracles remain
independent. Full-band N4 exact-v2 CM/HO trajectories match the independent DFT
fixtures within 5e-13, and same-build resumed state/ledger words match uninterrupted
execution. Actual rejection/refusal and corrupt-checkpoint controls also pass.

Informational scans record 116 duplicate blocks and 2.81% duplicate lines within
jscpd's 331 processed Rust files, and nine distinct unused-helper warning labels.
These findings are not a universal claim of absence. Mutation sweeps were not
rerun for the alpha. All original evidence remains available.

Daily CI runs the complete instrumented workspace/all-target suite once,
including numerical examples, plus stable build/install, Rustdoc and installed
CM/HO checkpoint/resume checks. This removes redundant executions of the same
long examples while retaining their coverage and numerical assertions.
No concentrating PDE window is accepted; coarse-grid agreement and a passing
quality gate do not establish spatial/force convergence.


The [first alpha publication](../evidence/runtime-alpha/publication/README.md)
now records successful source-matched hosted Rust and Python checks, including
all 220 Python tests. The downloaded hosted binary also passes both default
methods, same-build checkpoint/resume and all 1,660 bundled checksums. These
publication results refer to immutable source `63f9a14`; later code changes need
their own evidence and exact-source CI before a daily snapshot.

The [complete hosted results at `82c0796`](../evidence/quality-hosted-82/README.md)
now cover the six-trajectory exact-v2 family and optional reduced pointwise
evaluator: 451 Rust tests/probes, one compiled Rustdoc example, 98.56% line and
88.85% branch coverage, and maximum CRAP24.33594. Complexity and file-size maxima
remain within the required limits. Python passes all 220 tests. The archived
raw coverage, metrics and source inventories make these measurements inspectable.

The subsequent [physical consumer](../evidence/p09/v2-physical/README.md) and
[reduced sampled provider](../evidence/p09/reduced-provider/README.md) each pass
their focused numerical/quality checks and whole-source static/lint checks.
Their combined source still requires a complete hosted run before daily release.
These quality results do not accept a concentrating PDE window.

The [exact-v2 force-sampling family](../evidence/p09/v2-force-family/README.md)
passes four focused harness tests, its isolated allocation executable and the
separately executed example. Seven instrumented implementation/test files cover
666/681 executable lines and 35/42 branches (97.80%/83.33%); the example remains
in static analysis but was not replayed a second time under LLVM. Across all eight
focused Rust files, maxima are CC13, cognitive10, Halstead53.2895, physical-file338
and CRAP13.8222. Focused strict Clippy, formatting and whole-workspace Rustdoc pass.
The original combined validation function's measured CRAP26.3231 failure is
retained in the evidence narrative; its separated final implementation passes.
Source-matched hosted workspace coverage and package checks remain pending.
