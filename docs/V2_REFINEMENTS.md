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

## Actual physical comparisons

`v2_experiment::physical::PhysicalFamilyPlan` attaches a bounded physical
consumer to the already admitted `FamilyPlan`. Construct a
`PhysicalFamilyWorkspace` from that plan and call `measure(&family)` once after
each successful `V2Family::advance`. The consumer compares the borrowed actual
states through the reusable `PhysicalComparisonWorkspace`; it does not create a
second family or pass the smooth `CyclicSine` forcing problem into exact-v2.

Each `PhysicalRefinementSample` contains four complete physical quantities in
fixed order: velocity, velocity gradient, velocity Hessian and vorticity. Every
quantity includes the same five pairs as the Fourier sample: N0/N1, N1/N2,
H0/H1, H1/H2 and CM/HO. The report retains the actual clock, V2 family
identity, diagnostic sample layout, configured relative floors and sampled RMS,
peak and relative statistics for each finding. The physical comparisons include
the complete finer Fourier band, all ordered tensor entries and the physical
curl orientation.

Admission reserves the comparison scratch, metadata, one report record and the
joint family-plus-consumer storage. It also reserves a finite allowance and
charges complete attempts, including calls refused before numerical traversal.
The workspace reuses its derivative and reduction scratch sequentially, so no
complete Hessian family is retained. A failed call consumes its charge and emits
no partial report; the schedule advances only after all twenty findings succeed.
Identity, manifest, accepted-clock and branch-clock checks reject stale or
unrelated families. Measurement leaves every integrated state, history and
family digest unchanged.

These are sampled numerical RMS and peak measurements on the configured physical
grid. They are not continuous supremum bounds and do not certify refinement or
PDE acceptance. This consumer reports global comparisons only; pressure,
regional aggregation, reference tracking, force sufficiency and arithmetic
evidence remain separate studies.

## Actual pressure comparisons

`v2_experiment::pressure::PressureFamilyPlan` adds a bounded pressure consumer
to the six actual V2 branches. `PressureFamilyWorkspace::measure(&family)` is
called after each successful `V2Family::advance`; it reports pressure and its
full three-component physical gradient for all five family pairs. The consumer
transfers each branch to the finest retained grid, forms conservative products
on the complete doubled grid, and compares both scalar quantities through reused
physical scratch. The report retains the accepted clock, family identity,
source/force/sample layouts and both relative floors.

The pressure force is produced independently by a fresh original `V2Force` on
the doubled finest grid. It uses the exact accepted clock and remains
unprojected; no stage RHS, analytical pressure, reduced provider or reference
field is supplied. Integration force sampling remains the fixed family `M=12`;
the pressure diagnostic force uses its explicit doubled-grid layout and is a
separate admitted policy.

Joint admission includes provider, conservative-product, comparison, buffer,
workspace and report storage. Each attempt charges one force evaluation, ten
conservative assemblies and ten physical comparisons, for 133 scalar transforms
with the V2 provider. Charges happen before validation; failures retain their
full charge, produce no partial report and do not advance the schedule. Borrowed
family states and histories remain unchanged. These are global sampled pressure
statistics with a mean-zero gauge, not continuum bounds, refinement
certification or PDE acceptance.

Because every pair uses the same accepted clock and independently evaluated
prescribed force, that common force contribution cancels from pair differences.
The pair oracle therefore checks nonlinear pressure differences, while the
separate force-only Poisson control checks force sign, full-band retention and
the mean-zero mode. Neither check establishes force-sampling sufficiency.
Reference-gauge, regional and accepted-artifact qualification remain separate.

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
quantum 2^-20 and endpoint 128, corresponding to the exact startup profile's
physical endpoint t = 1/8192. It prints the case/family identity, finite
resource bounds, Fourier differences and the physical velocity/gradient/
Hessian/vorticity RMS and peak findings. This is a short startup diagnostic,
not the first concentrating endpoint t = 1/256 and not an accepted window. The
default CLI alpha remains the separately documented
[single-trajectory profile](RUNTIME_ALPHA.md).

Focused implementation and quality evidence is retained in
[evidence/p09/v2-physical](../evidence/p09/v2-physical/README.md).
