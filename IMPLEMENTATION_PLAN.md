# NSBU Solver implementation plan

Revision 1.4 · 11 September 2026 · First runtime alpha released; scientific packages through P07 verified

## Adopted decisions

Build a standalone Rust library and CLI named **NSBU Solver**, hosted in `nivatechnologies/nsbu-solver`. License original project code and documentation under **Apache-2.0**. The numerical baseline is [COMPLETE_DESIGN.md revision 0.7](docs/design/COMPLETE_DESIGN.md), with revision 0.8 review and release decisions preserved under `docs/design/`.

This document is the active repository plan. It supersedes the historical roadmap's project naming and proposed `MIT OR Apache-2.0` license. It does not alter the frozen equations, benchmark parameters, arithmetic contracts, or acceptance criteria. The [historical detailed fixtures](docs/design/IMPLEMENTATION_PLAN.md) remain normative where consistent with these explicit naming, licensing, and release-scope decisions.

The initial release implements periodic 3D incompressible Navier–Stokes at fixed positive viscosity and the separately defined `similarity-mms-v2` manufactured case. Literal source reproduction is excluded from this release. The mathematics compiler and a possible averaged-stress surrogate do not block the runtime. Visualization follows numerical validation. An optional Niva adapter is maintained outside this public workspace.

## Runtime-first execution priority · 11 September 2026

The user has prioritized a complete usable solver before extended numerical
qualification and visualization. The next delivery is the **runtime alpha** below.
It does not complete P10 or change any scientific acceptance criterion. All existing
implemented and verified work is retained; additional evaluator optimizations and
concentrating refinement infrastructure are parked until this workflow is usable.

| Order | Runtime-alpha work | Concrete exit evidence |
|---|---|---|
| R01 | Connect the existing exact-v2 force and CM/HO integrators to an owned from-rest run with explicit plan, clock, tolerances, work and diagnostics | Existing independent N4 trajectory fixtures still agree; both methods run without reference assignments; actual refusals preserve committed state |
| R02 | Bind complete same-profile checkpoint/resume to that owned run | Interrupted/resumed execution reproduces the uninterrupted next attempt and endpoint; exact input/configuration and all retained ledgers survive; corrupt/foreign payloads are refused and external origin remains explicit |
| R03 | Expose dry-run, execution, checkpoint/resume and readable machine output through the public CLI | Bounded end-to-end walkthrough for both methods, meaningful failure exits, immutable case identity and diagnostic status |
| R04 | Finish public installation/API/usage documentation and the runtime release evidence | Fresh public checkout installs and runs the documented workflow; current required quality gates pass; advertised features match executed commands |

Run focused numerical and transactional checks while integrating. Run the complete
workspace/coverage/quality and packaging gates at the runtime milestone and after
changes that invalidate their results. Do not repeat full suites merely to produce
another incremental evidence package. Preserve the existing metric thresholds;
mutation/dead/duplication findings remain informational.

Defer the new reduced-variable evaluator, additional performance backends,
large concentrating refinement families, extended arithmetic qualification and
visualization. P08/P09 scientific integration and P10 remain open until their
original exits pass. A functional runtime and a qualified concentrating result
are reported as separate deliverables. The existing smooth runtime remains usable.

Publish the first solver alpha as soon as R01-R04 pass. Thereafter publish daily
prereleases containing new changes that pass the runtime/CI gates, with source
revision, checksums, changes, test results and known limitations. Continue
correctness studies and optimization after that first release; completion of
scientific qualification and eventual stability remain separately demonstrated.
The runtime checklist R01–R04 is now complete, with
[actual exit evidence](evidence/runtime-alpha/README.md): 434 Rust tests/probes,
98.56% line and 88.79% branch coverage, all required metric limits, and a fresh
checkout package/install/CM/HO/restart walkthrough. The `alpha_release.ready`
flag is true. Both hosted workflows passed on the exact release commit, and
[the first alpha](evidence/runtime-alpha/publication/README.md) is published with
a verified downloaded binary. The [first daily prerelease](evidence/runtime-alpha/daily-20260911/README.md)
now passes exact-source hosted gates and downloaded-binary checks, including
464 Rust tests/probes. P08/P09/P10 scientific exits remain open.

## Repository bootstrap and current evidence

The bootstrap supplies README, license and attribution files, installation/use documentation, contribution guidance, the public reviewed baseline, exact case inputs, executable design checks, and CI configuration. A passing bootstrap check is not completion of a Rust numerical milestone.

The [alpha.1 release](evidence/runtime-alpha/alpha-20260911-2/README.md) is now
published from `88015d7`, with both source-matched hosted suites and independent
downloaded-binary checks passing. It includes the seven-event `diagnose-v2`
command. [Hosted evidence](evidence/p09/hosted-alpha1-88015d7/README.md) supersedes
earlier combined-CI-pending notes for that frozen source; scientific package
exits remain open and the N8/N12/N16 first-endpoint pilot is spatially unresolved.

The Rust workspace and help/version CLI exist. P03 spectral operators passed independent direct-sum/convolution checks, allocation instrumentation and all quality gates locally and in hosted CI. P04 provides CM steps, bounded attempts and transactional commits, with all package checks passed locally and in hosted CI. P05 independent Rust scalar/jet fields and the bounded force provider passed all local and hosted gates, completing P05. The P06 Rust N=4 exact-v2 diagnostic reaches 1/256 from rest and passes its independent 80/120-digit direct-DFT trajectory comparison. Smooth temporal and grid studies and all local and hosted quality gates pass, completing P06. P07 adds the independent HO method, nonautonomous order studies and CM/HO concentrating comparisons; all local and hosted checks pass, completing P07. P08 implements independent full-band comparisons, conservative double-grid pressure/residual diagnostics, sampled/local/regional reporting, reconstruction and accepted-history balance studies. Reporting and balance gates pass locally and hosted. The current diagnostics/window-review code passes local and hosted gates; full experiment/provenance integration remains pending. The bounded CLI runs both CyclicSine (`smooth`) and exact-v2 (`v2`) diagnostics with CM or HO, with complete same-profile runtime checkpoint continuation. R01–R04 alpha exits pass locally; see the runtime evidence above. No admitted source instance, accepted concentrating PDE window, or formal proof build exists. Coarse Python CM/HO exact-v2 trajectories now reach the first endpoint from rest with large tracking errors; P00C diagnostic and quality checks passed locally and in hosted CI. Independent Python pointwise, region and N=4 smooth from-rest step/trajectory fixtures are implemented, with temporal refinements. P00B numerical and quality gates passed locally and in hosted CI. P01 packaging, fresh-checkout installation and all Rust quality gates also passed locally and in hosted CI. P02 exact clocks, layouts, resource ledger and owned-state checks passed with full line/branch coverage and zero surviving mutants, locally and in hosted CI. N=8/12 arithmetic studies passed; exact-v2 force sampling completed with unresolved spatial differences. The current machine-readable state is [project-status.json](project-status.json). Future changes update that file only with linked execution evidence.

## Implemented public organization

The workspace has three crates. The following map describes current components;
see [architecture](docs/ARCHITECTURE.md) for their ownership and extension rules:

```text
Cargo.toml                 public workspace; Apache-2.0 metadata
Cargo.lock                 exact dependency resolution committed
rust-toolchain.toml         exact tested Rust version and components
crates/nsbu-solver/         public library facade
  src/domain/              geometry, layout, modes, state, exact clock
  src/spectral/            FFT, padding, projection, curl, pressure
  src/integrators/         ETD coefficients, stages, attempt/commit
  src/diagnostics/         physical quantities and sampled residuals
  src/experiment/          bounded control and recorded transactions
  src/checkpoint/          physical/history encoding and integrity
  src/verification/        finite empirical policies and review
  src/lineage/             ancestry, transfers and invalidation
crates/nsbu-benchmarks/     force/reference evaluators and owned diagnostic runs
crates/nsbu-cli/            binary name nsbu; I/O and experiment orchestration
reference/                 independent Python scalar/jet and direct-DFT path
crates/*/tests/fixtures/   small, versioned, independent numerical fixtures
benchmarks/                immutable case definitions
tools/                     current design and repository checks
docs/                      installation, usage, reviewed design, evidence policy
```

The library does not depend on the CLI, reference evaluator, viewer, source compiler, or any adapter. A prescribed-force trait can evaluate only its declared mathematical input. The integrator never receives an interface that can assign the reference field into its state. Comparison code owns reference access separately. Keep pure operators independent of filesystem and scheduling code.

The implemented FFT uses caller-owned radix workspaces and explicit normalization,
with independent direct-DFT and convolution fixtures. Rust 1.94.0 and the public
`num-complex` and `sha2` dependencies are pinned in the toolchain and lockfile.
Changes to the arithmetic backend or native dependencies require explicit
licensing, resource, determinism and numerical verification evidence.

## Ordered implementation packages

Each row should become a focused change or a small group of changes. A package is complete only when its exit evidence is checked into a small fixture or linked report and its prerequisites hold.

| ID | Depends on | Work | Required exit evidence |
|---|---|---|---|
| B00 | Reviewed baseline | Public repository bootstrap, license, docs, preserved inputs, check runners and CI | Local repository checks and design checks pass; installation limits stated; publication separately verified |
| P00 | B00 | Adopt baseline bytes and benchmark identity | Frozen hashes match; runtime/source/surrogate labels agree across active documents |
| P00B | P00 | Independent Python scalar root and implicit-jet evaluator; arbitrary-precision direct DFT and full small-grid step fixtures | Axis/cutoff/startup derivatives, residuals, force accuracy and 80/120-digit comparisons; no reused production FFT/kernel as oracle |
| P00C | P00B | Resource-qualified Python PDE diagnostic pilot and force-sampling studies | An independently evolved from-rest trajectory; allocation, error and failure reports; diagnostic status retained if unresolved |
| P01 | P00B | Create public Cargo workspace, select and pin toolchain/dependencies, package metadata, format/lint/test CI | A fresh public-only checkout builds/tests; package dry-run succeeds; no unpublished/private dependency; benchmark registration is not a PDE result |
| P02 | P01 | Domain/layout/state, checked sizes, typed errors, exact `u128` tick clock, plan epochs and memory preflight | Mode/conjugacy/Nyquist fixtures; quarter-stage tick tests and exhaustion refusal; buffer ownership, invalid input and cap/overflow tests |
| P03 | P02 | FFT abstraction, normalization, 3/2 padding/cropping, projection, curl, rotational nonlinearity and pressure | Direct DFT and explicit convolution agreement; deliberate aliases detected; mean force/velocity preserved; gradient force pressure fixture; semidiscrete inviscid energy identity |
| P04 | P03 | Cox–Matthews ETDRK4 coefficients/stages, one bounded attempt, external retry scheduler, transactional swap commit | High-precision coefficient comparisons including branch boundaries; constant source, diffusion and RK4 limits; actual stage-time force requests; stale-token refusal; repeated accept/reject allocation and rollback instrumentation |
| P05 | P02,P00B | Independent Rust scalar/jet force provider and reference evaluator for exact v2 input | Root/jet residuals, mixed derivatives, axis limits, cutoff and startup checks, pressure/divergence identities, force cancellation and accuracy reports |
| P06 | P03,P04,P05 | Smooth generic cases and first complete 3D concentrating diagnostic run from rest | Smooth time/space refinement; verified force coefficients; zero reference assignments; unresolved pilots retained with the correct status |
| P07 | P04,P06 | Hochbruck–Ostermann five-stage method with an independent coefficient implementation | Nonzero-operator tableau identities, stiff nonautonomous order study, nonmonotone force-request test, per-window comparison against Cox–Matthews |
| P08 | P03,P06 | Full-band/off-stage diagnostics, pressure and balance comparisons, window verifier | High-mode and force-aliasing negative controls; residual reconstruction independent of stage equations; sampled norms distinguished from rigorous supremum/enclosures |
| P09 | P06,P07,P08 | Separate force/reference/arithmetic/transfer studies and complete restart lineage | Same-problem identity, inherited error and invalidation replay; checkpoint next-attempt equivalence; restart-transfer comparison against direct fine evolution |
| P10 | P07,P08,P09 | First qualified endpoint and progressively closer benchmark endpoints | All mandatory refinements pass frozen tolerances for each claimed endpoint; rejected endpoints and the last qualified interval are saved |
| P11 | P01–P10 for concentrating results; see separate runtime gate below | Public library/CLI examples, installation, schemas, checkpoint tools and release evidence | Clean-checkout end-to-end walkthrough; documented exit codes; package/install dry-run; exact evidence supports every advertised capability |
| P12 | P10,P11 | Small read-only viewer | Viewer exposes case identity, accepted frontier and failed channels; cannot edit state or reclassify evidence |

P05 may proceed after P02 without waiting for the FFT implementation. P00C informs integration and sampling choices but is not a substitute for the Rust validation. Every PDE pilot requires memory preflight before allocation. Mathematical reference work is the first numerical implementation priority.

## Code review and measurable quality gates

Review existing maintained code before numerical expansion and review each package
before completion. Apply SOLID through single-purpose modules, narrow interfaces,
substitutable implementations with contract tests, extension through explicit
interfaces, and dependencies directed toward mathematical/domain abstractions.
Force evaluation, reference evaluation, state mutation, scheduling and I/O retain
separate responsibilities. Avoid speculative abstractions introduced solely to
satisfy a metric.

| Metric | Required threshold |
|---|---:|
| Cyclomatic complexity, per function/method | < 22 |
| Cognitive complexity, per function/method | < 22 |
| Halstead difficulty, per function/method and file | < 80 |
| Physical lines per maintained source/test file, including comments/blanks | < 500 |
| Test coverage, executable lines and branches | >=80% each |
| CRAP, per function/method | < 25 |
| Mutation, dead-code and duplication findings | Informational, with review |
| `any` or `unknown` types, including Python `Any` and unresolved implicit types | 0 |

These are required targets, not demonstrated current results. Before claiming a
quality gate passed, pin language-appropriate analysis/coverage/mutation tools,
record versions, commands, source revision, complete file scope and raw reports,
and wire the checks into CI. Use CRAP = CC² × (1 − coverage)³ + CC, with branch
coverage expressed as a fraction. Report each metric separately; averages cannot
hide a failing function or file. Unsupported measurements remain unverified.
Mutation reports include generated, killed, surviving, invalid, equivalent,
timed-out and untested counts. They inform review but do not gate package
completion. Document demonstrably equivalent mutants individually; never silently
exclude difficult code or weaken numerical tests to reach a target.

The scope includes maintained library, CLI, reference, tooling and test code.
Frozen review artifacts under docs/design/ are immutable historical inputs and
must not be rewritten to meet metrics; report their inventory separately.
Generated fixtures and third-party dependencies are separately inventoried, not
represented as analyzed project code. Any further exclusion requires explicit
justification recorded in the report, with unmet requirements left visible.
Independent numerical implementations are intentional verification oracles;
duplication findings remain informational and must not force their merger into a
shared implementation that would destroy independence. Dead/redundant-code
analysis must include a manual review of intended public API and
independent-oracle responsibilities.

Each package records a SOLID review, numerical exit evidence, quality measurements
and unresolved findings. A package cannot be marked complete with failing or
unmeasured applicable gates. Bootstrap mathematical checks alone do not satisfy
these code-quality gates or establish a validated PDE trajectory.

## First concrete changes after bootstrap

1. **Independent reference evaluator:** implement the exact rational v2 inputs, scalar admissible-root bracket, implicit derivatives, smooth cutoff/startup and scalar field/force evaluations in `reference/`. Include origin, collar, axis and decreasing-positive-time samples. Establish an independent high-precision error budget before exporting fixtures.
2. **Rust foundations and operators:** create the three-crate workspace and exact clock/state types, then add direct-DFT comparison fixtures before optimizing FFT or product code. Exercise mean modes and the pressure response to a gradient force explicitly.
3. **Bounded integration and from-rest pilot:** implement CM ETDRK4 with fully preallocated attempts and validated swap commits. Run smooth tests before a concentrating diagnostic. Save failed resolution channels, then add the second integrator and verifier before claiming any benchmark window.

Do not compress these into one large numerical implementation without independent fixtures. Changes to formulas or case definitions require a reviewed design amendment and new identities where applicable.

## Acceptance and failure policy

Every concentrating branch begins at `t=0`, `u=0` with the same immutable problem definition. The endpoint ladder is `t_k = (1/128)(1 - 2^(-k))`. Continuing from a checkpoint retains its complete error history and dependencies. Restarting from an analytical reference creates a separately labelled local test.

Before qualification, freeze tolerances and observables using preliminary studies. The required comparisons cover space, time, force evaluation/sampling, analytical reference, arithmetic, transfers and inherited errors. Use full fine-band and derivative-sensitive norms, physical pressure, balances, off-stage residuals, interior-region coverage, startup and cutoff-collar errors. Common-band agreement and a small sampled spectral tail cannot qualify an interval.

A failed channel rejects or limits the window. Refine that channel, reduce the interval, or report the failure. Do not clip velocity, change viscosity, remove unresolved force structure, reset the state, or switch mathematical input to obtain a passing plot. A changed problem gets a new identity. Numerical sampling cannot by itself prove singularity or slab-wide bounds.

Required negative controls include a divergence-free high mode, aliased force, reduced force precision, modified case identity, reference-seeded restart, transferred coarse checkpoint, rejected step and stale commit token. Each has a declared expected diagnostic response. See the [detailed fixtures](docs/design/IMPLEMENTATION_PLAN.md) for exact test requirements.

## Resource gate

The base design reservations are 1.25336, 9.98218, 79.67871 and 636.71484 GiB at `128^3`, `256^3`, `512^3` and `1024^3`. Add all FFT, force-provider, metadata, diagnostics, I/O and allocator storage before approving a plan. Any storage reuse requires a concrete lifetime schedule.

The twelve-cells-per-peak-radius screen permits at most 0, 1, 3 and 5 endpoints respectively on those individual grids. These are neither accepted endpoints nor guaranteed usable refinement families. Start pilots at an approved size and classify them by measured resolution. Larger grids are not an assumed available upgrade.

Before every numerical run, the proposed CLI dry-run must print exact target times, input identity, methods, force coverage, memory by allocation class, evaluation bounds and unsupported capabilities. The bounded profile refuses unbounded providers. Limits and failures are structured statuses rather than partial physical advances.

## Release gates

**Bootstrap:** documents and selected mathematical checks only. No solver version, binary release, crates.io publication or numerical capability claim.

**Generic runtime alpha:** P01–P04 and the generic-case portions of P06–P09 pass, including second-method temporal verification, operator fixtures, transaction/checkpoint tests and diagnostic negative controls. The library and CLI have tested clean-checkout installation and usable generic examples. An alpha may explicitly mark the concentrating verifier incomplete; it cannot advertise qualified concentrating results.

**Qualified concentrating-results release:** P05–P10 and the complete P11 workflow pass. Publish the immutable case and comparison protocol, tested execution profile, accepted/rejected endpoint records, numerical reports and the exact last qualified interval. A source-construction claim is never implied.

**Visualization release:** P12 follows a qualified result. A viewer is not a dependency of the runtime or its tests.

For every Rust release, run formatting, strict linting, unit/integration/doc tests, dependency-license review, package dry-run, CLI help/invalid-input checks and a fresh-install walkthrough. Commit `Cargo.lock`. Bitwise determinism is scoped to an explicitly recorded execution profile; cross-hardware agreement is a separate validation.

## Deferred source work

The construction compiler remains `MathematicsVerificationOnly`. The explicit slow-mesh policy remains `FeasibilityExcluded`; broader literal feasibility remains `FeasibilityUnestablished`. Reopening literal reproduction requires actual source objects, derivative/phase bounds, force coverage, representation-specific resource estimates and the construction-ledger admission gates. A prescribed averaged-stress experiment must receive its own model identity and plan.

Source extraction, a formal proof build, a source-instance compiler, an averaged-stress model and Niva integration are outside this implementation release's critical path. The project can deliver a useful independent Navier–Stokes runtime without claiming these deferred products exist.


## P08 implementation progress

The [core diagnostics](evidence/p08/core/README.md) and
[reporting/balance increment](evidence/p08/reporting/README.md) pass local and
source-matched hosted gates. The [window measurement review](evidence/p08/window/README.md)
passes 195 tests/probes, complete coverage of 9,733 executable lines and 810
instrumented branches, and 2,175 accounted-for mutations with zero survivors or
timeouts. It requires separate channel evidence, exact sampling refinements,
explicit off-stage reconstruction probes and a bounded observation schedule.
Hosted verification of this increment remains pending. **P08 is not complete.**

A numerical pass returns `ReadyForLineageReview`, never an accepted PDE window.
Remaining integration must bind the complete frozen benchmark observable inventory,
current-grid/same-problem comparisons and actual accepted-state evidence. P09 must
provide complete refinement/restart lineage and checkpoint history before P10 can
qualify any concentrating endpoint. The separate checkpoint/provenance draft does
not satisfy those package exits yet.


## P09 foundation progress

The [lineage and physical-image foundation](evidence/p09/foundations/README.md)
passes all local gates: 202 tests/probes, complete line/branch coverage, and 2,240
accounted-for mutants with zero survivors or timeouts. Its registry preserves
transfer/local-test ancestry and replays transitive force invalidation. Physical
images reproduce the next accepted/rejected CM and HO attempts with fresh scratch.

These components do not complete P09. Complete controller, diagnostic, error and
reconstruction history, coherent checkpoint assembly, external provenance and
same-problem refinement/transfer studies remain. The recorded-step and balance
history increment now passes its local gates; hosted Rust checks remain pending. No accepted concentrating window is implied.


## P09 recorded-step progress

The [recorded-step/controller/balance increment](evidence/p09/recorded/README.md)
passes all local gates: 221 tests/probes, 11,612 executable lines and 942 branches
fully covered, and 2,353 mutations with 2,171 caught, 182 unviable and zero
survivors/timeouts. Diagnostics and history are proposed before an infallible
physical commit. The fixed controller retains terminal failures; compensated
balance arithmetic retains pending Simpson samples. Complete recorded attempts
allocate nothing after preflight. All requested code metric limits pass.

**P09 remains incomplete.** Complete coherent checkpoint assembly still needs raw
measurements, reconstruction history, authenticated artifacts and provider
state/work accounting. Those and transfer-vs-direct-fine tests remain separate
drafts. The generic runtime and concentrating releases remain gated by the active
plan; this historical increment did not yet supply a numerical CLI or an accepted concentrating window.

The [artifact/replay increment](evidence/p09/artifacts/README.md) records 226
local tests/probes, a complete current Rust coverage replay and reproducible
checkpoint artifact checks. It is progress evidence only; **P09 remains incomplete.**

## P09 owned-runtime progress

The [bounded smooth runtime and binary components](evidence/p09/runtime/README.md)
pass local verification: 267 tests/probes, 99.20% executable line coverage, 95.02%
instrumented branch coverage and all required complexity/CRAP limits. The installed
`nsbu smooth` command and runnable library example evolve actual state from rest.
Versioned physical, history, artifact, lineage and smooth-owner formats enforce
bounded decoding and retain import-origin restrictions. A real forced-shear
transfer study verifies inherited error against independent direct fine evolution.

**P09 remains incomplete.** Complete reconstruction ownership/checkpoints, external
accepted-state provenance and full force/reference/arithmetic/transfer refinement
families remain. File save/resume commands are the next CLI increment. P08's
experiment integration and P10 qualification remain gated; no concentrating window
is accepted. Source-matched hosted Rust and Python checks for the owned-runtime increment pass.

## P09 accepted-reconstruction and file-I/O progress

The [owned reconstruction increment](evidence/p09/reconstruction/README.md)
passes 284 local tests/probes, 99.08% executable line coverage, 91.56% branch
coverage and all required complexity/CRAP limits. It adds smooth file save/resume,
transactional accepted-node publication and a shared owner supporting balance-only
and reconstruction observation profiles. Trusted reconstruction snapshots preserve
the next accepted/rejected CM/HO attempt and the off-stage interpolant. A fresh
source install passes 14 CLI scenarios. Source-matched hosted Rust and Python checks pass.

The binary format still supports only the balance-only smooth profile. Binary
reconstruction persistence, full external accepted-state provenance, independent
refinement families and the complete P08 experiment integration remain. P09 is
incomplete, and P10 has no qualified concentrating endpoint.

## P09 reconstruction archive progress

The [coherent reconstruction archive](evidence/p09/reconstruction-archive/README.md)
passes 292 local tests/probes, 99.08% executable line coverage, 91.44% branch
coverage and all required complexity/CRAP gates. Its outer integrity frame binds
accepted reconstruction to the same physical state, raw history, controller and
work ledger. Imported runs preserve the next CM/HO attempt and interpolant with
fresh scratch and an unverified origin. Source-matched hosted Rust and Python checks pass.

The library format does not establish external artifact semantics or complete
accepted-state provenance. Those bindings and independent refinement families
remain; P08/P09 and concentrating qualification are incomplete. CLI file commands
continue to select the balance-only smooth profile.

## P08/P09 independent experiment progress

The [smooth experiment and actual-replay increment](evidence/p09/experiments/README.md)
passes 305 local tests/probes, 99.01% executable line coverage, 91.23% branch
coverage and all required complexity/CRAP limits. It owns six independent rest
trajectories, produces complete-band synchronized comparisons, measures genuine
off-stage reconstruction/defects and reproduces coherent checkpoint bytes through
a fresh bounded numerical evolution. A clean public source export runs the
documented example and installs the CLI. Source-matched hosted Rust and Python checks pass.

P08/P09 remain incomplete. Frozen full-observable/time inventory binding, external
artifact semantics, independent force/reference/current-grid arithmetic studies,
sampling/quadrature and concentrating integration remain. Matching smooth replay
does not qualify a PDE window or authenticate an external execution artifact.

## P09 same-grid smooth arithmetic progress

The [independent arithmetic increment](evidence/p09/arithmetic/README.md) passes
313 Rust tests/probes and 184 Python tests, all required quality gates, fresh
packaging and a clean-source workflow. Sixteen independently evolved direct-DFT
trajectories cover N=4/N=12, CM/HO, 80/120 digits and separate fixed-force inputs.
Full coefficient discrepancies, exact input identities and observed precision
separation are retained. The largest same-input Rust/120-digit coefficient
discrepancy is about 4.807e-18. Source-matched hosted Rust and Python verification pass.

This closes the smooth velocity arithmetic implementation increment on the
current N=12 family grid, not P09 or a concentrating window. Complete frozen
observable/time/policy binding, all-observable studies, external artifact
semantics and concentrating integration remain. Numerical and serialization
responsibilities are separate; malformed inputs and unmodified high-mode/reality
defects have explicit regression coverage.

## P08 canonical protocol progress

The [canonical numerical protocol](evidence/p08/protocol/README.md) passes 321
local Rust tests/probes, 98.93% executable line coverage, 91.29% branch coverage
and all required complexity/CRAP gates. It binds every channel rule, ordered key,
exact tested-time manifest and reconstruction geometry; canonical output and
bounded review share the admitted immutable settings. Short-buffer and zero
post-admission allocation contracts pass. Fresh-target packaging and bootstrap
checks pass; source-matched hosted Rust and Python verification pass.

P08 remains incomplete. The generic identifier does not establish benchmark
observable semantics or accepted-state provenance. Complete local derivative,
pressure, sampling and all-observable experiment production, benchmark-specific
inventory binding and P09 lineage integration remain. No PDE window is accepted.

## P08 physical derivative progress

The [physical derivative increment](evidence/p08/derivatives/README.md) passes
332 Rust tests/probes and 188 Python tests. It adds complete-band first/second
physical derivatives, scalar/vector/Frobenius tensor errors, v2 regional tensor
reports and 80/120-digit independent analytical derivative fixtures. Actual CM/HO
rest trajectories retain nonzero derivative errors and unchanged integrated
states during sampling. All required local quality gates, fresh packaging,
clean-source installation and frozen mathematical checks pass; source-matched hosted Rust and Python checks pass.

P08 remains incomplete. Full benchmark observable/time/policy binding, complete
all-observable experiment production, pressure mean/reference refinement and
accepted-state/P09 provenance integration remain. Pointwise derivative agreement
and small smooth trajectories do not qualify a concentrating window.

## P08 complete physical comparison progress

The [physical-field comparison increment](evidence/p08/physical/README.md) passes
341 Rust tests/probes, 98.93% executable line coverage, 91.05% branch coverage and
all required complexity/CRAP limits. It measures complete scalar/vector/gradient/
Hessian/vorticity differences with sequential scratch, preserves mean and fine-only
mode errors, and feeds identical samples to v2 regional reports. Actual independent
CM/HO rest states retain nonzero method differences and unchanged field digests.
Fresh-source public tests, packaging and frozen bootstrap checks pass. The unchanged
Python source retains its complete 188-test evidence. Source-matched hosted Rust
and Python verification pass.

P08 remains incomplete. The experiment must bind these measurements to the full
frozen benchmark inventory, exact time/policy and accepted-state provenance.
Complete pressure/reference, force/arithmetic/transfer refinements and concentrating
integration remain. No concentrating PDE window is accepted.

## P08 actual physical refinement-family progress

The [six-trajectory physical increment](evidence/p08/physical-family/README.md)
passes 345 Rust tests/probes, 98.93% executable line coverage, 90.72% branch
coverage and every required complexity/CRAP gate. It binds complete velocity,
gradient, Hessian and vorticity refinements to actual synchronized independent
rest states, exact policy words and a finite observation allowance. Joint
admission and failed-attempt charges pass allocation/refusal checks. The clean
public example, fresh packaging and frozen bootstrap checks pass; the unchanged
Python source retains its complete 188-test profile. Source-matched Rust and Python hosted checks pass.

P08/P09 remain incomplete. Complete pressure/reference/force/arithmetic/sampling
and all-observable evidence, full benchmark protocol/artifact binding and
concentrating integration remain. No concentrating PDE window is accepted.

## P08 actual pressure refinement-family progress

The [pressure-family increment](evidence/p08/pressure-family/README.md) passes
350 Rust tests/probes, 98.90% executable line coverage, 90.68% branch coverage and
all required quality gates. Full doubled-band physical pressure and its gradient
come from all six actual smooth states and independently prescribed force.
Analytic gauge/high-band controls, bounded failures and allocation probes pass.
The updated jointly admitted example, clean source, packaging and frozen bootstrap
checks pass; the unchanged Python inventory retains its 188-test profile.
A measured draft CRAP failure and its corrected complete replay remain recorded.
Source-matched Rust and Python hosted checks pass.

P08/P09 remain incomplete. Full all-observable reference/force/arithmetic/sampling
studies, benchmark/accepted-artifact binding, concentrating pressure mean/regional
studies and complete window production remain. No concentrating window is accepted.

## P09 complete smooth derived-field arithmetic progress

The [derived-field arithmetic increment](evidence/p09/derived-arithmetic/README.md)
passes 355 Rust tests/probes, 198 Python tests, every required quality gate and
clean-source reproduction. Four N=4/N=12 CM/HO studies independently compare all
ordered velocity tensors, vorticity and full mean-zero pressure/gradient, with
separate same-state, same-force and target-force effects. Every observed 80/120
precision ratio is below 1e-40. Source hashes, complete inputs and measurements
are retained; source-matched Rust and Python hosted checks pass.

P08/P09 remain incomplete. Error norms in this study are reduced in Python;
production physical-reducer arithmetic, balances/residual/regional/location
channels, complete artifact binding and concentrating current-grid qualification
remain. No accepted concentrating PDE window is supplied by these smooth studies.

## P08 actual physical sampling-refinement progress

The [sampling-family increment](evidence/p08/sampling-family/README.md) passes
362 Rust tests/probes, 98.82% executable line coverage, 90.24% branch coverage
and every required quality gate. It retains six complete quantities, five actual
trajectory pairs and three physical sample grids at each accepted clock. Joint
resources, failed-attempt charges, terminal child failures, unchanged state
digests and an analytic missed-peak control are verified. The clean source
reproduces all 270 public-example comparisons byte-for-byte; unchanged Python
retains its complete 198-test profile. Source-matched hosted Rust and Python checks pass.

P08/P09 remain incomplete. Arbitrary off-stage time manifests require streaming
reconstruction before bounded histories are overwritten. Complete reference,
force, production-reduction arithmetic, quadrature, regional/location and
benchmark/artifact studies and concentrating qualification remain. No window
is accepted from near-zero smooth sampling differences.

## P08 streaming accepted-history probe progress

The [probe-family increment](evidence/p08/probe-family/README.md) passes 367 Rust
tests/probes, 98.81% executable line coverage, 90.11% branch coverage and every
required complexity/CRAP gate. It streams early and late exact physical probes
before bounded accepted histories are overwritten. Six independent rest owners
retain their original fixed steps; separate full-band value/derivative scratch
preserves actual node origins. Final state words equal a separately evolved
family. Admission, rejection, stale-history and allocation controls pass, as do
clean-source reproduction, packaging and frozen/bootstrap checks. Unchanged
Python retains the complete 198-test profile. Source-matched hosted Rust and Python checks pass.

P08/P09 remain incomplete. Complete physical/reference/force/residual consumers,
reconstructed-time refinement qualification, benchmark/artifact binding and
concentrating current-grid studies remain. No interpolant replaces an integrated
state and no concentrating PDE window is accepted.

## P09 global reference-pressure gauge progress

The [reference-gauge increment](evidence/p09/reference-gauge/README.md) passes
209 Python tests, 99.80% executable line coverage, 98.89% branch coverage and all
required typing/complexity/CRAP gates. Nine bounded 80/120-digit studies cover
rest, startup and the first two endpoint times, with separate axial/collar
refinements through 512 panels and an exact Gaussian radial core. The finest
nonzero relative quadrature changes are about 1.6–1.7e-11; overlapping profiles
reproduce exactly. Clean-source replay, 41 bootstrap tests and frozen mathematical
checks pass. Unchanged Rust retains its verified 367-test/probe profile. Source-matched hosted Rust and Python checks pass.

P08/P09 remain incomplete. These empirical means are independent reference
inputs, not enclosures or window tolerances. Every future claimed probe requires
its own reference budget, full regional/all-channel integration and actual
concentrating trajectory convergence. Accepted concentrating windows remain zero.

## P08 complete reconstructed physical-field progress

The [physical-probe increment](evidence/p08/probe-diagnostics/README.md) passes
372 Rust tests/probes, 98.79% executable line coverage, 90.25% branch coverage and
all required complexity/CRAP gates. It binds thirty complete velocity/tensor/
pressure comparisons to each exact reconstructed probe, retaining actual node
origins and a fresh unprojected force at the physical time. Unchanged state and
interpolant digests, rest/clock controls, terminal child failure and allocation
contracts pass. Clean-source output, packaging and frozen/bootstrap checks pass;
unchanged Python retains its complete 209-test profile. Hosted Python passes; hosted Rust passes numerical checks but fails on a prose-only lexical type match, repaired in the following increment.

P08/P09 remain incomplete. Complete residual/reconstruction, reference/force/
arithmetic/regional/location refinement, benchmark/artifact binding and
concentrating current-grid qualification remain. No concentrating window is
accepted from these smooth physical observations.


## P08 streamed residual progress

The [streamed-residual increment](evidence/p08/streamed-residuals/README.md) passes 378 Rust tests/probes
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

The [reduction-arithmetic increment](evidence/p09/reduction-arithmetic/README.md) passes 385 Rust tests/probes
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

The [balance/quadrature increment](evidence/p08/balance-probes/README.md) passes
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

The [force-evaluation increment](evidence/p09/force-evaluation/README.md) passes
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

The [parallel-force increment](evidence/p09/parallel-force/README.md) passes
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

## Post-alpha exact-v2 refinement progress

The [independent exact-v2 family](evidence/p09/v2-family/README.md) evolves six
private rest trajectories with fixed force sampling across retained grids. Its
full-band space/time/method comparisons pass an independent signed Fourier-sum
oracle, versioned identity checks, allocation probes and terminal-state controls.
Eight focused tests/probes pass; all eight new Rust files measure 98.71% line and
85.42% branch coverage with maximum CRAP19.6133. Whole-workspace static limits,
strict Clippy and Rustdoc pass. Source-matched hosted gates pass for this increment.

The documented N=[4,8,12], M=12 example reaches only the short startup time
1/8192. P08/P09/P10 remain incomplete with zero accepted concentrating windows.
The [physical consumer](evidence/p09/v2-physical/README.md) now measures complete
velocity, gradient, Hessian and vorticity differences across all five actual
pairs. Independent signed Fourier sums check sampled RMS and peaks; clock and
identity failures spend bounded attempts without changing integrated states.
Three physical tests, an allocation probe and the example pass, with four
retained admission/identity tests also exercised. Focused coverage is 97.77%
line and 84.48% branch with maximum CRAP19.6133; current static/lint/Rustdoc
checks pass. Source-matched hosted gates passed at `6170341`.
The coarse-grid Hessian differences increase under spatial refinement; there
is no accepted convergence claim. Next add independent pressure diagnostics
and complete separate force/reference/arithmetic and accepted artifact studies.
Daily releases require passing current CI.

## Post-alpha reduced-coordinate evaluator progress

The [optional pointwise evaluator](evidence/p09/reduced-force/README.md) passes
nine earlier fixtures plus 84 exact-word 80/120-digit cases, independent symbolic
transport identities, complete monomial/composition checks and allocation probes.
Across its ten new Rust files, focused coverage is 98.35% line and 92.86% branch
with maximum CRAP16. All current static/lint/documentation limits pass. A fresh
source export reproduces fixture bytes and numerical profile output, passes the
focused tests and packages all three crates. Full source-matched CI passes at
`82c0796`, including the original and reduced pointwise evaluators.

The measured pointwise speed ratio is about six; no runtime acceleration or new
PDE qualification is established. The original runtime provider remains unchanged.
The [optional reduced sampled provider](evidence/p09/reduced-provider/README.md)
now passes complete Fourier/direct-DFT and 80/120-digit fixture comparisons,
cache/clock/work contracts and allocation checks. Seventeen focused tests/probes
pass, with 98.75% line and 90.625% branch coverage and maximum CRAP19.125 across
the added/changed implementation. The N16/M24 complete-provider profile measures
a median speed ratio of 6.48 with maximum scaled coefficient difference 2.35e-14.
Full source-matched CI passed at `6170341` for this provider increment. The
[independent trajectory checks](evidence/p09/reduced-trajectories/README.md)
now pass for CM and HO on N4/M4 to 1/256: every committed coefficient agrees
between providers, all three components at the 18 explicitly listed strict-band
endpoint modes (excluding Nyquist slots) match the independent direct-DFT
fixtures, and exact clocks/work/transactions remain correct. Both new tests pass
with 271/271 covered executable lines and CRAP6; LLVM emits no branch records for
these test files, and the full workspace branch gate remains a hosted release
requirement. No trajectory timing or spatial convergence is established. Next
bind explicit arithmetic identity before changing runtime selection.
P08/P09/P10 retain their original incomplete status.
The optional reduced sampled provider remains outside runtime selection. Its
[focused evidence](evidence/p09/reduced-provider/summary.json) is supplemented by
[hosted release evidence](evidence/runtime-alpha/daily-20260911/summary.json).
Both hosted workflows also passed at `c8a3872`, including the later independent
trajectory tests (Rust run 34574045422; repository run 34574045397).

## P09 exact-v2 force-sampling trajectory refinement

A [bounded first-endpoint feasibility study](evidence/p09/first-endpoint-pilot/README.md)
using the published alpha binary reaches `t=1/256` from rest for N8/N16 and
M16/M32 with 64-tick CM steps. Endpoint norms and balance defects change materially
under grid refinement. This is diagnostic timing/resource evidence, not a
full-field refinement study or an accepted window; use fixed worker counts in
the next controlled family comparison.

The [bounded force-sampling family](docs/V2_FORCE_REFINEMENTS.md) independently
evolves three N4 CM trajectories from rest while changing only the nested
prescribed-force grid M=[4,8,16]. Its exact manifest, versioned family identity,
aggregate storage/work/attempt admission, terminal failure retention and
allocation-free execution are tested. Independent signed full-complex sums check
both complete retained-band velocity and derivative-sensitive comparisons, with
a deliberate ownership/self-comparison negative control. The post-startup endpoint is 1/2048;
measured differences are retained without a sufficiency or convergence decision.
P08/P09/P10 remain incomplete and accepted concentrating windows remain zero.

## P09 exact-v2 pressure consumer

The [pressure consumer](evidence/p09/v2-pressure/README.md) reports global
mean-zero pressure and gradient differences for all five actual family pairs.
A separate full-band prescribed-force control checks the contribution that
cancels in pair differences. Focused numerical/allocation gates pass, with
96.92% line coverage, 86.11% branch coverage and maximum CRAP15. The earlier
CRAP30 failure and corrected source replay remain recorded. Combined-source
hosted checks are pending; regional/reference/arithmetic/artifact integration
and concentrating qualification remain incomplete. Zero PDE windows are accepted.

## P09 exact-v2 analytical trajectory tracking

The [analytical tracking consumer](docs/V2_REFERENCE_TRACKING.md) observes all
six synchronized exact-v2 family states without mutation. One cached reference
evaluation per clock/grid point supplies velocity, complete gradient, ordered
Hessian and curl targets; three reusable derivative workspaces sample every
actual retained band. Joint storage, reference/root/FFT/traversal work, attempts,
foreign/stale refusal, terminal publication and allocation contracts pass focused
checks. Independent signed Fourier reconstruction checks all global RMS/peak/
relative statistics, and the existing 120-digit reference fixture remains within
its tolerance. The coarse startup endpoint has nonzero errors, including N12/h16
CM velocity RMS 1.8874e-7 and Hessian RMS 9.3256e-4. This partial global sampled
study does not qualify pressure, regions, current-grid arithmetic or a PDE window.
P08/P09/P10 remain incomplete with zero accepted concentrating windows.

## P09 exact-v2 regional analytical tracking

The [regional tracking consumer](docs/V2_REGIONAL_TRACKING.md) reuses completed
actual-versus-analytical magnitude arrays from all six accepted family states.
It retains the unchanged global velocity, ordered gradient/Hessian and curl
findings plus Core, Annulus, InteriorOutsideNominal, Collar and Exterior sampled
classes. Separate joint storage and complete classification/root/visit work are
admitted before evaluation; refusal and state-integrity contracts are tested.
Missing sampled classes remain explicit rather than measured zero. These classes
do not claim completeness for independently selected, potentially overlapping
nominal coverage sets. Pressure, gauge, current-grid arithmetic bounds,
continuum bounds and convergence remain open; zero PDE windows are accepted.

## P09 exact-v2 accepted-node binding

The [accepted-node bridge](docs/V2_NODE_BINDING.md) compares ordinary-family
accepted states bit-for-bit with exact nodes still retained by the separate
lookahead probe family. Stored clock, epoch and accepted-step provenance must
match before a complete six-branch record is published. Evicted or not-yet-made
nodes remain explicit missing evidence, and neither later current state nor an
interpolant is substituted. Joint two-family storage, ring lookup and two-sided
coefficient visits are separately admitted and charged before validation. This
bridge adds provenance for later diagnostic composition; it supplies no bound,
tolerance, convergence or PDE-window decision.

## P09 exact-v2 accepted-node reconstruction owner

The [reconstructed owner](evidence/p09/v2-reconstruction/README.md) independently
retains three accepted endpoint value/RHS nodes using the shared transactional
observer kernel. The default exact-v2 runtime and smooth archive format are
preserved. Initial-rest work is separately charged; failed admission allocates
nothing and rejected steps leave the accepted ring unchanged. Focused smooth
and exact-v2 regressions pass, with 95.43% lines, 84% branches and CRAP19.125.

The subsequent [six-owner probe increment](evidence/p09/v2-probes/README.md)
now streams early and late exact-v2 off-stage values and physical-time derivatives,
with actual accepted-node origins and whole-report publication. Its focused
numerical, resource and quality checks pass: 96.43% line coverage, 84.62% branch
coverage and maximum CRAP 17. Residual assembly, binding to the independently
owned accepted-state diagnostic family, and external reconstruction import remain
open. Combined-source hosted checks are pending; no PDE window is accepted.

## P09 exact-v2 off-stage residual family

The [residual consumer](docs/V2_RESIDUALS.md) measures six complete doubled-band
momentum defects from genuine non-stage reconstructed fields, using fresh
original exact-v2 forcing on one common doubled force-sampling grid. Independent
late-mode convolution, force-sign and nonlinear-omission controls pass;
[focused evidence](evidence/p09/v2-residuals/summary.json) records bounded work,
zero steady allocations, 94.5455% line / 80% branch coverage and maximum CRAP 16.
The strengthened [probe oracle](evidence/p09/v2-probe-oracle-correction/README.md)
uses the correct configured diagnostic force grid and records the withdrawn
intermediate result. Combined hosted checks remain pending. Complete refinement channels and concentrating qualification remain
open; P08/P09/P10 are incomplete and accepted PDE windows remain zero.

## P09 exact-v2 current-grid reference arithmetic

The [guarded reference study](evidence/p09/reference-arithmetic-full/README.md)
completed all 5,184 rows on the 12-cubed sample grid at ticks 0/64/128, comparing
independent 80/120-digit evaluations, rational versus exact binary64 inputs, and
Rust output words. Velocity, gradient, ordered Hessian and curl are covered.
The largest Rust/120-digit component discrepancy was 1.052e-17; the retained
80/120-digit diagnostic passed at 6.262e-83. Focused numerical and quality gates
pass, with raw source-bound evidence and the invalid earlier partial retained.
This is empirical reference-evaluator arithmetic, excluding pressure/gauge,
integrator arithmetic and continuum bounds. P08/P09/P10 remain incomplete.

## P09 exact-v2 reconstructed balance quadrature

The [balance/quadrature consumer](docs/V2_BALANCE_QUADRATURE.md) measures six
original-force reconstructed balance streams and three nested Simpson levels
over 3/5/9 clocks. All six late-clock fresh force-profile controls agree;
hand-weighted actual-sample and independent polynomial quadrature checks pass.
[Focused evidence](evidence/p09/v2-balance-quadrature/summary.json) records
97.568% line / 80.769% branch coverage, maximum CRAP 15.568, bounded work and
zero steady allocations. Combined hosted validation and complete concentrating
qualification remain pending; P08/P09/P10 are incomplete.

## P09 bounded exact-v2 diagnostic coordination

The [coordinator](docs/V2_DIAGNOSTIC_COORDINATOR.md) owns two independently
evolved six-branch families and publishes a complete seven-clock startup
manifest. Accepted-state spectral, physical, pressure, regional-reference and
node-binding findings remain distinct from off-stage residual findings. Full
raw results agree with standalone consumers on the identical actual states.
[Focused evidence](evidence/p09/v2-diagnostic-coordinator/summary.json) records
93.590% line / 80% branch coverage, maximum CRAP 16.042, bounded storage/work
and zero steady allocations. Ten missing channels remain explicit. The
coordinator applies no acceptance policy; the adapter to the complete numerical
and lineage review is still pending. Combined hosted validation is pending,
and P08/P09/P10 remain incomplete with zero accepted PDE windows.

## P09 nested physical sampling

The [sampling consumer](docs/V2_SAMPLING.md) holds all six integrated states
fixed while measuring four complete physical quantities on nested 12/24/48
lattices. It retains all five pair comparisons, separate RMS and peak values,
and the first sample location attaining each maximum. Nondivisible lattices
are rejected before allocation. Numerical, allocation and per-function CRAP
checks pass. [Evidence](evidence/p09/v2-nested-physical-sampling/summary.json)
reports library coverage separately from the 78.26% focused branch coverage
including test assertions; the complete maintained-source release gate remains
pending hosted CI. Pressure sampling and complete window review remain open.

## P09 diagnostic CLI and configured workers

The fixed `nsbu diagnose-v2 [--dry-run]` command exposes the bounded startup
coordinator, with case/profile identities, separate quantity summaries and
explicit missing channels. [CLI evidence](evidence/p09/v2-diagnose-cli/summary.json)
includes its changed example in the focused coverage/CRAP scope. Pressure and
residual diagnostics now honor the configured original-force worker count
without changing their sample grids; [worker evidence](evidence/p09/v2-diagnostic-workers/summary.json)
records bitwise serial controls and complete changed-function CRAP checks.
Combined hosted validation remains required. P08/P09/P10 remain incomplete.

## P09 first-endpoint family pilot

A [bounded N8/12/16 family pilot](evidence/p09/v2-first-endpoint-diagnostic/manifest.json)
completed five accepted/off-stage reports from rest through tick4096 (`1/256`)
in 36 minutes, under a 45-minute timeout. Fixed force sampling was M16, with
three time settings and independent CM/HO branches. The [full-band extraction](evidence/p09/v2-first-endpoint-diagnostic/spectral-full-norms.json)
shows spatial H1 differences increasing from34.19 to39.17; temporal and method
differences are much smaller. This exposes unresolved spatial behavior rather
than accepting a window. The frozen execution source predates parallel
pressure/residual consumers; its exact harness and raw output are retained.
