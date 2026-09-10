# Architecture

NSBU Solver is a Rust workspace for a periodic, incompressible
three-dimensional Navier--Stokes discretisation and for evaluating the evidence
produced by bounded numerical experiments. It is an implementation in progress:
the public CLI supports bounded smooth diagnostics, and no PDE convergence window has
been accepted. The current command-level boundary is described in
[usage](USAGE.md); the scientific claims and their limits are in
[scientific scope](SCIENTIFIC_SCOPE.md).

This document describes implemented public-library components. The
reviewed material in [design](design/COMPLETE_DESIGN.md) is a frozen engineering
baseline, rather than a code-current API manual.

## Workspace and public boundaries

The workspace has three public crates:

| Crate | Responsibility | Boundary |
|---|---|---|
| [`nsbu-solver`](../crates/nsbu-solver/README.md) | State, spectral operations, bounded integration, diagnostics, empirical review, lineage, and experiment transactions | Does not provide a numerical CLI, complete checkpoint format, or a PDE acceptance decision. |
| [`nsbu-benchmarks`](../crates/nsbu-benchmarks/README.md) | Independent `similarity-mms-v2` scalar/jet reference and prescribed force | Reference APIs cannot modify state; `smooth_run` owns a separate smooth diagnostic trajectory. |
| [`nsbu-cli`](../crates/nsbu-cli/README.md) | Installed `nsbu` command entry point | Exposes smooth preflight, runs and unverified file continuation; concentrating qualification remains incomplete. |

The project has no Niva dependency. A provider implements the solver's bounded
RHS/force contracts; it is deliberately separate from an analytical reference
evaluator and from state ownership.

## Library flow

```text
Domain, Layout, TickClock, ResourcePlan
                 |
                 v
       SpectralState + preallocated workspaces
                 |
                 v
 force/RHS -> spectral operator -> CM or HO bounded attempt
                 |                         |
                 |                         +--> candidate and local indicators
                 v
      independently evaluated diagnostics ----> empirical review
                 |                                      |
                 +--> accepted-history measurements     +--> lineage review input
```

`domain` establishes the immutable geometry, representation, exact clock,
resource plan, epoch, and independently allocated from-rest state. `spectral`
uses caller-owned FFT/operator workspaces to transform, project, differentiate,
and evaluate the rotational nonlinearity. `integrators` performs a requested
CM or HO full/two-half attempt using a separately admitted RHS.

An attempt produces either an empirical local acceptance token or a rejection.
It does not itself mutate the committed `SpectralState`. The token is single use;
the transaction code verifies plan, state, epoch, and candidate identities before
a commit. The experiment layer can commit a physical proposal and a measured
balance-history update together. These guarantees concern state consistency, not
spatial, temporal, force, or arithmetic convergence.

The `diagnostics` module borrows fields and clocks to make independent
measurements. `verification` reviews supplied, finite empirical measurements
against a frozen policy; its strongest positive result is a review finding, not
a qualified PDE window or an error enclosure. `lineage` records bounded
ancestry and invalidation independently of numerical qualification.

## Ownership and extension rules

* Construct a complete `ResourcePlan` before allocating a state or workspace.
  A plan is tied to one `Domain` and `Epoch`; it is not a measurement of resident
  memory. See [resources and errors](RESOURCES_AND_ERRORS.md).
* Treat `SpectralState` as committed physical state. Scratch belongs to a
  workspace, and returned borrowed views may be invalid after an error or when
  their workspace is reused.
* Implement force/RHS code with declared finite storage, work, and transform
  costs. A reference field must never initialize or replace the evolving state.
* Keep diagnostics read-only and separately preflighted. Measurements of finite
  grids or times must retain their sampling and arithmetic limitations.
* Preserve exact clocks, identities, and accepted/rejected history when forming
  a continuation or comparison. Lineage declarations do not authenticate input
  files or turn a record into an accepted result.

The modules are intentionally public at a relatively fine granularity. Their
Rustdoc gives each type's immediate contract; this guide supplies the ownership
and sequencing context needed to compose them. The crate-level API begins at
[`nsbu_solver`](../crates/nsbu-solver/src/lib.rs), with the public module roots
in [`domain`](../crates/nsbu-solver/src/domain/mod.rs),
[`integrators`](../crates/nsbu-solver/src/integrators/mod.rs),
[`diagnostics`](../crates/nsbu-solver/src/diagnostics/mod.rs),
[`verification`](../crates/nsbu-solver/src/verification/mod.rs), and
[`lineage`](../crates/nsbu-solver/src/lineage/mod.rs).

## Owned observation profiles

`SmoothRun` and `ReconstructedRun` are concrete aliases of the same `OwnedRun<O>`
implementation. The sealed observation extension supplies complete reservations,
construction and trusted snapshot restoration for the two built-in profiles. It
preserves a single integrator/controller/transaction path. The public solver's
observer trait remains the narrow interface for other user-supplied observers.

`ReconstructionObserver` keeps three accepted value/derivative nodes and one private
proposal. Measurement independently evaluates the conservative RHS on a double grid;
no integrator-stage derivative is reused. The physical transaction and prepared raw
history commit before the infallible observer publication callback. Failed proposals
discard pending data while retaining their charged work. The owner exposes only a
shared observer reference, so callers cannot publish an unrelated proposal into an
owned run. Trusted snapshots copy accepted nodes and counters and rebuild scratch;
the balance-only binary format cannot silently drop active reconstruction.

## Independent smooth experiment producers

`smooth_experiment::FamilyPlan` admits six complete runs before any numerical
allocation. `SmoothFamily` privately owns those independent from-rest trajectories
and returns full-band spatial, temporal and method differences only after their
clocks agree. A failed branch terminates the family without erasing prior commits.
Reconstruction and residual workspaces separately borrow accepted histories;
neither receives mutable physical state or integrator scratch. The residual force
and conservative products are evaluated independently at the exact off-stage probe.
The [experiment guide](EXPERIMENTS.md) gives branch identities, storage/work contracts,
error behavior, source navigation and a runnable example.

`smooth_run::replay::ReplayPlan` separately admits a complete from-rest numerical
replay of a reconstruction checkpoint. It compares canonical physical, diagnostic,
controller and work bytes after independent evolution. The input remains borrowed
and unverified; successful output owns the newly integrated rest trajectory.
This is reproducibility evidence under the executing smooth provider, not external
artifact authentication or PDE qualification.

## Work that remains

`verification::protocol::FrozenProtocol` borrows validated policies and exact time
and reconstruction manifests. A single canonical traversal feeds either SHA-256
or a caller-owned byte buffer. Every channel's effective settings participate in
identity, and new reviews use the same immutable inputs. The generic component
cannot infer complete benchmark semantics from observable keys; the benchmark
adapter must bind the required inventory and actual measurements separately.
The [format guide](PROTOCOL_FORMAT.md) documents byte layout, work admission and
the independent cross-language fixture.

The separate [arithmetic workflow](ARITHMETIC_STUDY.md) compares owned smooth
trajectories against allocating Python direct sums. Its input decoder, immutable
force fixture, trajectory scheduler, report reader and numerical comparator have
separate responsibilities. The reference receives no Rust state during evolution.
Exact raw force bits cross the implementation boundary only in the explicitly
labeled fixed-input study. Imported evolved states are read without projection or
conjugate-pair repair. The workflow reports measured arithmetic and force effects;
it has no authority to accept a PDE window.

Binary components and a balance-only smooth-owner archive are implemented.
Smooth file save/resume and owned reconstruction snapshots are implemented.
Binary reconstruction serialization is available through a separate Rust owner
codec. Complete artifact/lineage binding, concentrating checkpoint assembly and
the full experiment verifier remain in progress.
An in-memory physical image and restartable controller/history components do
not amount to a complete checkpoint. See the implemented-versus-planned
sections of [usage](USAGE.md) and the active
[implementation plan](../IMPLEMENTATION_PLAN.md) for release criteria.

Physical derivative diagnostics reuse a separately reserved scalar FFT workspace;
velocity, tensor and pressure components are processed sequentially. Generic
fixed-storage tensor errors and v2 regional geometry have separate responsibilities.
The analytical derivative evaluator cannot access integrated state. See
[derivative diagnostics](DERIVATIVE_DIAGNOSTICS.md) for coordinate, normalization,
pressure-gauge and independent-fixture contracts. Complete all-observable window
production and qualification remain in progress.

[Complete physical-field comparisons](PHYSICAL_COMPARISONS.md) compose scalar
sampling and complete tensor reductions without storing every derivative grid.
The v2 adapter reuses the existing geometric partition and preserves all global
samples. This numerical layer does not own state evolution or authenticate
problem/time provenance; the complete experiment remains responsible for those
bindings and its finite observation schedule.
