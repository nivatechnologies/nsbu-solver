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
| [`nsbu-cli`](../crates/nsbu-cli/README.md) | Installed `nsbu` command entry point | Exposes smooth preflight/runs; concentrating qualification remains incomplete. |

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

## Work that remains

Binary components and a balance-only smooth-owner archive are implemented.
Complete concentrating checkpoint assembly, file-based CLI restart and the full
experiment verifier remain in progress.
An in-memory physical image and restartable controller/history components do
not amount to a complete checkpoint. See the implemented-versus-planned
sections of [usage](USAGE.md) and the active
[implementation plan](../IMPLEMENTATION_PLAN.md) for release criteria.
