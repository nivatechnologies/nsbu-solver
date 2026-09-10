# Resources and errors

The solver uses explicit preflight to make the memory and bounded-work contract
visible before numerical grid allocation. Passing preflight approves declared
storage; it neither measures process memory nor proves that a provider has
declared every allocation. The root [resource table](../README.md#resources)
contains conservative base reservations and explicitly excludes several caller
supplied costs.

## Resource planning

Create `ExtraStorage` and then `ResourcePlan::new(domain, extra, cap, epoch)`
before constructing state or workspaces. Its eight byte classes are, in order:

| Class | Contents |
|---|---|
| 1 | Retained spectral vectors |
| 2 | Padded real arrays |
| 3 | Padded complex arrays |
| 4 | Tables |
| 5 | FFT plans and scratch |
| 6 | Force evaluator storage |
| 7 | Diagnostics, history, and output staging |
| 8 | Metadata and an explicit allocator-overhead allowance |

The first four are derived from the validated domain; the last four are caller
declarations. The `total` is a checked sum and must not exceed `cap`.
`ResourcePlan::classes` exposes the same ordering for workspace admission. The
precise accounting is in
[`ResourcePlan`](../crates/nsbu-solver/src/domain/resources.rs).

Each component adds its own reservation. For example, reserve selected-method
attempt storage with `AttemptWorkspace::reservation_with_method`, force storage
from the provider's declared bounds, and diagnostics/history from their own
reservation functions. State buffers, input/output buffers, FFT planning,
diagnostic grids, output staging, and allocator overhead must all be included
where applicable. A plan that is sufficient for a CM attempt need not be
sufficient for HO or a conservative double-grid diagnostic.

After admission, the library intends transforms, operator evaluations, bounded
force requests, and attempts to reuse owned scratch rather than allocate. An
allocator can nevertheless refuse an approved reservation. Callers should keep
the plan's `Epoch` with the objects it authorized; a plan from another domain or
generation cannot safely be reused.

## Bounded work

An admitted RHS reports storage, point/root work, scalar transforms, and
invocation limits. Unknown costs cannot enter the bounded spectral profile.
The solver charges force work across every RHS call in an attempt, including
full/two-half CM or HO work. A work unit is provider-defined and finite; it is
not a count of floating-point operations or a wall-clock promise.

Diagnostics and verification also require finite caller capacities. This avoids
turning a missing observation, unlimited retry, or failed geometric
classification into an implicit unlimited computation. The relevant contracts
are in [`forcing`](../crates/nsbu-solver/src/integrators/forcing.rs),
[`attempt`](../crates/nsbu-solver/src/integrators/attempt.rs), and
[`verification`](../crates/nsbu-solver/src/verification/mod.rs).

## Error handling and state safety

Public fallible operations return structured errors. `SolverError` is documented
variant-by-variant in
[`error.rs`](../crates/nsbu-solver/src/error.rs); callers should match errors at
the level appropriate to their policy rather than treat every refusal as a
numerical rejection.

| Error family | Typical response | State effect |
|---|---|---|
| Invalid domain/index/clock/step/payload/spectrum | Correct input or configuration. | No committed-state change. |
| `SizeOverflow`, `ResourceLimit`, `AllocationFailed` | Reduce scope, increase a declared cap, or fix reservation accounting. | No committed-state change. |
| Arithmetic/clock capacity limits | Choose representable time/grid parameters; do not silently round exact geometry. | No committed-state change. |
| Provider cost/budget failures | Correct or replace the provider's declaration/implementation. | No committed-state change. |
| `StaleAttempt` | Discard the token and build a new candidate from current committed state. | No committed-state change. |
| Retry/advection limits | Follow the externally declared scheduler policy; do not reinterpret as convergence. | No implicit successful commit. |

An `Ok` result can still contain an ordinary local rejection, so inspect the
attempt result and optional acceptance token. Only a valid token can commit a
candidate. The experiment-level `commit_balanced` operation checks state/history
identities before it swaps physical state and history together; a failure
consumes the token but changes neither. See
[`experiment`](../crates/nsbu-solver/src/experiment/mod.rs).

`VerificationError` and `LineageError` are separate because their concerns are
finite evidence review and ancestry declaration, respectively. Neither an
accepted local attempt, successful review input, nor lineage record establishes
physical provenance, a complete checkpoint, or a qualified PDE window.

## Current operational limit

There is no public numerical run, resume, or checkpoint-file command yet.
In-memory state, image, controller, and history components should not be
presented as a durable complete checkpoint. Use the commands and limitations in
[usage](USAGE.md) when exercising the current checkout, and consult the
[implementation plan](../IMPLEMENTATION_PLAN.md) for future release work.
