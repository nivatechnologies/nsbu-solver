# Exact-v2 comparison families

The `v2_experiment` library module runs six independently owned trajectories of
the exact `similarity-mms-v2` prescribed-force problem. Its samples report actual
full-band spatial, temporal and method differences at synchronized exact times.
It does not issue a qualified-window status. The complete force, arithmetic,
reference, physical/local, reconstruction and quadrature studies remain required.

## Branches and admission

`FamilySettings` specifies three strictly increasing retained grids, three
strictly decreasing nested steps, one shared force sample grid and worker count,
an endpoint, local integration tolerances and an advective guard. The physical
problem remains the unit cube with viscosity one. Retained grids must satisfy
the solver's four-multiple domain requirement. All runs start at exact zero;
there is no import or analytical-state assignment interface.

| Slot | Grid | Step | Method |
|---:|---|---|---|
| 0 | N0 | h2 | CM |
| 1 | N1 | h2 | CM |
| 2 | N2 | h2 | CM |
| 3 | N2 | h0 | CM |
| 4 | N2 | h1 | CM |
| 5 | N2 | h2 | HO |

The force sample grid M and worker selection stay fixed across every branch.
Changing N therefore changes retained trajectory resolution without also changing
force sampling. This separation does not establish that M is sufficient; force
resolution and precision require separate studies. Each branch independently
owns its integration provider, doubled-grid balance observer, state, controller
and spent-work history.

`FamilyPlan::new` takes immutable `TestedTimes` and a byte cap. The sample manifest
starts at rest, ends at the declared endpoint and aligns with every branch's
step. Its exact tick representation must describe T* = 1/128. Admission sums all
six complete run reservations, family metadata, integration and observer work,
full comparison work across all sample times, and the canonical identity traversal.
The caller owns manifest and retained report storage and must budget those separately.

## Samples and failure behavior

`V2Family::advance` evolves each branch to the next exact sample time. Comparisons
are N0/N1, N1/N2, h0/h1, h1/h2, and CM/HO at N2/h2. Every primary difference
includes the complete finer band. Common-band and newly resolved contributions
are reported separately, along with preserved mean differences and derivative
norms. No phase alignment or recentering is performed.

Any child rejection, refusal or numerical error terminates the family. Branches
retain their own already committed states and charged attempts; there is no
all-branch rollback or implicit retry with fresh allowances. Read-only branch
access permits inspection of this partial progress. No sample is emitted until
all branches reach the required time and every comparison succeeds.

## Identity and limits

The sample's family SHA-256 binds the immutable case, branch settings and exact
sample manifest. Version 1 encodes `NSBUV2FAMILY0001`, the 64 ASCII case-hash bytes,
u128 little-endian grid/step values, force half-length and dimensions, workers
and endpoint, five binary64 words (absolute/relative tolerances and guard), the
u128 time count and all clocks (i32 exponent plus u128 target/elapsed/remaining).
The fixed physical problem and branch order are part of this version's semantics.
The hash is streamed without allocating a serialization buffer; its byte length
is checked and reported before hashing.

This identifies a diagnostic family configuration. It does not replace the
complete frozen observable/policy protocol, authenticate an external trajectory,
or prove convergence. Current samples contain no analytical tracking error and
cannot fill missing channels with zero. See the
[active implementation plan](../IMPLEMENTATION_PLAN.md) and
[scientific scope](SCIENTIFIC_SCOPE.md).

## Example

```sh
cargo run --release -p nsbu-benchmarks --example v2_refinement
```

The example preflights N = [4,8,12], fixed M = 12 and steps [64,32,16] with tick
quantum 2^-20 and endpoint 128. It prints its case/family identity, finite
resource bounds and measured full-band differences. This is a short startup
diagnostic, not the first concentrating endpoint t = 1/256 and not an accepted
window. The default CLI alpha remains the separately documented
[single-trajectory profile](RUNTIME_ALPHA.md).
