# NSBU Solver implementation plan

Revision 1.1 · 9 September 2026 · Bootstrap, P00B/P00C and P01/P02 verified

## Adopted decisions

Build a standalone Rust library and CLI named **NSBU Solver**, hosted in `nivatechnologies/nsbu-solver`. License original project code and documentation under **Apache-2.0**. The numerical baseline is [COMPLETE_DESIGN.md revision 0.7](docs/design/COMPLETE_DESIGN.md), with revision 0.8 review and release decisions preserved under `docs/design/`.

This document is the active repository plan. It supersedes the historical roadmap's project naming and proposed `MIT OR Apache-2.0` license. It does not alter the frozen equations, benchmark parameters, arithmetic contracts, or acceptance criteria. The [historical detailed fixtures](docs/design/IMPLEMENTATION_PLAN.md) remain normative where consistent with these explicit naming, licensing, and release-scope decisions.

The initial release implements periodic 3D incompressible Navier–Stokes at fixed positive viscosity and the separately defined `similarity-mms-v2` manufactured case. Literal source reproduction is excluded from this release. The mathematics compiler and a possible averaged-stress surrogate do not block the runtime. Visualization follows numerical validation. An optional Niva adapter is maintained outside this public workspace.

## Repository bootstrap and current evidence

The bootstrap supplies README, license and attribution files, installation/use documentation, contribution guidance, the public reviewed baseline, exact case inputs, executable design checks, and CI configuration. A passing bootstrap check is not completion of a Rust numerical milestone.

The Rust workspace and help/version CLI exist. No Rust numerical solver, numerical CLI, admitted source instance, accepted PDE window, or formal proof build exists at this point. Coarse Python CM/HO exact-v2 trajectories now reach the first endpoint from rest with large tracking errors; P00C diagnostic and quality checks passed locally and in hosted CI. Independent Python pointwise, region and N=4 smooth from-rest step/trajectory fixtures are implemented, with temporal refinements. P00B numerical and quality gates passed locally and in hosted CI. P01 packaging, fresh-checkout installation and all Rust quality gates also passed locally and in hosted CI. P02 exact clocks, layouts, resource ledger and owned-state checks passed with full line/branch coverage and zero surviving mutants, locally and in hosted CI. N=8/12 arithmetic studies passed; exact-v2 force sampling completed with unresolved spatial differences. The current machine-readable state is [project-status.json](project-status.json). Future changes update that file only with linked execution evidence.

## Planned public organization

Start with three crates rather than a crate for every module. The three crate roots now exist; numerical module paths below remain planned:

```text
Cargo.toml                 public workspace; Apache-2.0 metadata
Cargo.lock                 exact dependency resolution committed
rust-toolchain.toml         exact tested Rust version and components
crates/nsbu-solver/         public library facade
  src/domain/              geometry, layout, modes, state, exact clock
  src/spectral/            FFT, padding, projection, curl, pressure
  src/integrators/         ETD coefficients, stages, attempt/commit
  src/diagnostics/         physical quantities and sampled residuals
crates/nsbu-benchmarks/     force providers and separate reference evaluators
crates/nsbu-cli/            binary name nsbu; I/O and experiment orchestration
reference/                 independent Python scalar/jet and direct-DFT path
fixtures/                  small, versioned, reproducible numerical fixtures
benchmarks/                immutable case definitions
tools/                     current design and repository checks
docs/                      installation, usage, reviewed design, evidence policy
```

The library does not depend on the CLI, reference evaluator, viewer, source compiler, or any adapter. A prescribed-force trait can evaluate only its declared mathematical input. The integrator never receives an interface that can assign the reference field into its state. Comparison code owns reference access separately. Keep pure operators independent of filesystem and scheduling code.

Choose an FFT implementation after a small licensing, normalization, scratch-planning, and deterministic-execution spike. Do not silently add a native dependency or incompatible license. Pin the actual tested dependency versions in P01; this plan does not invent a future Rust toolchain version.

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
| Test coverage, executable lines and branches | 100% |
| CRAP, per function/method | < 25 |
| Surviving mutants | 0 |
| Dead code | 0 |
| Redundant code | 0 |
| `any` or `unknown` types, including Python `Any` and unresolved implicit types | 0 |

These are required targets, not demonstrated current results. Before claiming a
quality gate passed, pin language-appropriate analysis/coverage/mutation tools,
record versions, commands, source revision, complete file scope and raw reports,
and wire the checks into CI. Use CRAP = CC² × (1 − coverage)³ + CC, with branch
coverage expressed as a fraction. Report each metric separately; averages cannot
hide a failing function or file. Unsupported measurements remain unverified.
Mutation reports include generated, killed, surviving, invalid, equivalent,
timed-out and untested counts. Timeouts and untested mutants do not count as
kills. Document demonstrably equivalent mutants individually; never silently
exclude difficult code or weaken numerical tests to reach a target.

The scope includes maintained library, CLI, reference, tooling and test code.
Frozen review artifacts under docs/design/ are immutable historical inputs and
must not be rewritten to meet metrics; report their inventory separately.
Generated fixtures and third-party dependencies are separately inventoried, not
represented as analyzed project code. Any further exclusion requires explicit
justification recorded in the report, with unmet requirements left visible.
Independent numerical implementations are intentional verification oracles;
review apparent duplication without merging them into a shared implementation
that would destroy independence. Dead/redundant-code analysis must include a
manual review of intended public API and independent-oracle responsibilities.

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
