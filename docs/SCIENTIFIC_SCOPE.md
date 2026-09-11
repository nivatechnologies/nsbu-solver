# Scientific scope

NSBU Solver is a standalone periodic three-dimensional incompressible Navier–Stokes runtime project with fixed positive viscosity and prescribed forcing. The released Rust library and CLI run bounded smooth and exact-v2 trajectories from rest with two time integrators, resource preflight, diagnostics and checkpoint/resume. The independent Python reference supplies scalar/jet evaluations and high-precision direct-DFT fixtures. Concentrating trajectory qualification remains incomplete; zero PDE windows are accepted. See the [runtime guide](RUNTIME_ALPHA.md) and [published release evidence](../evidence/runtime-alpha/daily-20260911/README.md).

The first experiment, `similarity-mms-v2`, uses a manufactured concentrating field and its independently prescribed force. Its purpose is to test independent from-rest integration over progressively closer finite intervals. It does not implement the source construction's pulse cascade or claim a force smoothly extended through the target time. Its concentration is part of the prescribed benchmark, not a newly discovered singularity.

## What this is useful for

Think of the benchmark as a controlled numerical experiment. We prescribe a
concentrating velocity field and derive the external force that makes it satisfy
the equations. The numerical solver then starts from rest and advances its own
velocity state using that force. Comparing its result with the analytical field
helps expose discretization error; the reference never resets the computed state.

Computational physicists can use this alpha to inspect spectral operators,
compare CM and HO time integration, reproduce small-grid diagnostics, and test
checkpoint continuation. The default N=4/M=4 run is a smoke-test-sized example:
reaching its endpoint does not mean it resolves the concentrating flow. Refining
the velocity grid, force sampling, time step and arithmetic separately is still
necessary. This alpha has no qualified concentrating convergence result, general
geometry/solid-wall workflow, or validated engineering turbulence application.

## Relation to the Millennium problem

The official problem allows proofs of global existence and smoothness **or**
breakdown under specified conditions. Its existence formulations use zero
external force; its breakdown formulations permit forcing with precise global
smoothness and decay conditions. Arbitrarily driving a benchmark harder does
not establish those conditions. See the [official Clay problem statement,
formulations A–D](https://www.claymath.org/wp-content/uploads/2022/06/navierstokes.pdf).

A finite numerical run cannot establish behavior at arbitrarily small scales or
at a singular time. Our manufactured case has no demonstrated smooth forcing
extension through its target time, and this release does not reproduce or verify
the cited manuscript's full construction. It provides numerical experiments and
explicit error diagnostics, with mathematical proof obligations kept separate.

## Product and evidence boundaries

| Product | Current state | Permitted claim after its own checks pass |
|---|---|---|
| Rust runtime | Released diagnostic alpha; tested small-grid profiles | Numerical accuracy on the tested equations, cases, intervals and execution profiles |
| Manufactured concentrating experiment | Exact case specified; no PDE windows accepted | Independently evolved, empirically qualified tracking of that exact benchmark on reported finite intervals |
| Construction compiler | Optional, `MathematicsVerificationOnly` | Named identities, inequalities or finite-stage residuals actually checked |
| Literal source reproduction | Outside this release; general feasibility unestablished | No current reproduction claim |
| Averaged-stress surrogate | Separate decision deferred | A distinct model with a separate identity and validation scope, if later commissioned |

The explicit slow-mesh strategy is `FeasibilityExcluded` under the finite-resource policy recorded in [source-feasibility-policy.json](design/source-feasibility-policy.json). That conditional statement is not a proof that every possible representation is excluded. See the [source assessment](design/SOURCE_FEASIBILITY.md) and [construction ledger](design/CONSTRUCTION_LEDGER.md) for the exact assumptions and remaining obligations.

## Required scientific discipline

Use one immutable mathematical problem per lineage. Preserve all accumulated errors when extending or restarting. Changing force, viscosity, initial data, domain or cutoff creates a different problem. A reference-seeded test receives an explicit local-test classification. Clipping, state replacement, unrecorded filtering or closure changes cannot be hidden inside a solver improvement.

Qualified windows require full-band and derivative-sensitive comparisons, independent temporal checks, force-sampling and arithmetic studies, pressure/balance/residual diagnostics and explicit startup/collar coverage. Sampled tails, matching coarse modes, successful compilation and larger grids are insufficient by themselves.

Determinism claims are scoped to a recorded compiler, dependency, transform, arithmetic and reduction profile. Cross-hardware agreement requires separate tests. Runtime error estimates and empirical residuals are distinguished from rigorous enclosures. No visualization or finite-sample fit can establish blow-up.

## Research provenance

The adopted documents reference the supplied [Navier–Stokes manuscript](https://cdn.openai.com/pdf/32d9f210-8b73-45e0-91bc-82a30aef8a9a/navier-stokes.pdf), with precise equation and lemma references where used. The repository preserves those assessments; it does not claim a new full-paper proof audit. Source PDF byte provenance and a Lean proof build remain unverified in the adopted evidence.

The [Euler visualization repository](https://github.com/pmocz/euler-blowup-viz) is a separate referenced project. No code from it is included in this bootstrap. The public NSBU project must remain independent of all optional downstream adapters.
