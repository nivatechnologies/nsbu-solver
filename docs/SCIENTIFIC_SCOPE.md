# Scientific scope

NSBU Solver is a standalone periodic three-dimensional incompressible Navier–Stokes runtime project with fixed positive viscosity and prescribed forcing. Its current artifacts are a reviewed design and selected executable mathematical checks. The Python reference now supplies pointwise field checks and N=4 smooth from-rest time-step fixtures. The Rust solver and concentrating trajectory validation remain future work.

The first experiment, `similarity-mms-v2`, uses a manufactured concentrating field and its independently prescribed force. Its purpose is to test independent from-rest integration over progressively closer finite intervals. It does not implement the source construction's pulse cascade or claim a force smoothly extended through the target time. Its concentration is part of the prescribed benchmark, not a newly discovered singularity.

## Product and evidence boundaries

| Product | Current state | Permitted claim after its own checks pass |
|---|---|---|
| Rust runtime | Planned | Numerical accuracy on the tested equations, cases, intervals and execution profiles |
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
