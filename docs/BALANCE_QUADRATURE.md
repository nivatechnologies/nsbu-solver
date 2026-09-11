# Reconstructed balance samples and independent quadrature refinement

`BalanceProbes` measures conservative energy and enstrophy balance terms from all
six independently reconstructed smooth trajectories. `BalanceQuadrature` consumes
those complete samples on three strictly nested physical-time schedules. The
integrated states and original macro steps remain unchanged while quadrature
spacing is refined. This separates quadrature sensitivity from trajectory and
reconstruction error.

## Execute the public example

```sh
cargo run --release -p nsbu-benchmarks --example smooth_balance_quadrature -- --dry-run
cargo run --release -p nsbu-benchmarks --example smooth_balance_quadrature
```

The CyclicSine profile uses retained N=4/8/12, macro steps H=64/32/16 ticks,
quantum 2^-16, endpoint tick 128, viscosity 1 and independently integrated CM/HO
branches from rest. Physical probes are 0/7/16/32/48/64/80/96/112/128. The three
quadrature schedules are:

| Level | Exact ticks | Maximum Simpson half-panel |
|---|---|---:|
| Coarse | 0/64/128 | 64 ticks |
| Middle | 0/32/64/96/128 | 32 ticks |
| Fine | 0/16/32/48/64/80/96/112/128 | 16 ticks |

Probe 7 is measured and reported with its original accepted-node history. It does
not belong to these quadrature schedules and cannot change their accumulated
sample counts. Each output probe includes every branch's actual node triple and
current lookahead state clock. The final report retains all eighteen integrals,
six per level. Earlier per-probe origins remain required output evidence; the
final integral record alone is not complete window provenance.

## Independent conservative measurements

Each balance observer has its own prescribed-force provider, doubled-band
conservative product workspace and full diagnostic fields. It receives only the
read-only reconstructed velocity, source domain and exact physical probe clock.
The fresh force is unprojected and evaluated at that clock. No integration-stage
RHS or analytical field is assigned to an integrated state or used to replace a
measured balance sample.

The reused independent Parseval balance kernels retain velocity/H1/vorticity/
divergence norms, energy, enstrophy, viscous energy dissipation, forcing work,
vortex stretching, enstrophy dissipation and vorticity forcing. The stretching
identity requires divergence-free velocity; the divergence measurement remains
visible. Pressure and force above the doubled diagnostic band require their own
studies and are not controlled by this measurement.

Each schedule uses compensated composite Simpson accumulation of
`forcing_work - energy_dissipation` and
`stretching + vorticity_forcing - enstrophy_dissipation`. A complete result retains
both integrals and the corresponding endpoint energy/enstrophy changes minus
those integrals. Pending midpoint samples cannot be discarded or presented as a
completed Simpson pair. Normalization is by physical volume, as in the underlying
balance kernels.

## Admission and transaction boundaries

`BalanceProbePlan` binds the complete original `ProbePlan`. Changed physics,
missing/skipped probes and mismatched full manifests are refused before child
numerical work. Each attempted observation consumes its full conservative
aggregate allowance. Child ledgers separately retain actual sample/provider/
transform charges. Malformed requests may be corrected only while attempts remain;
a child numerical failure is terminal and publishes no partial six-branch record.

`QuadraturePlan` additionally requires three exact schedules spanning the same
from-rest window. They must be strict nested sample sets, all contained in the
owner manifest. Each schedule has an odd number of at least three points, every
Simpson pair has equal exact half-spans, and the maximum half-span strictly
shrinks at both refinements. Merely adding points without reducing that maximum
is refused. Admission bounds every membership, nesting and geometry scan before
reading through the caller's manifests.

`BalanceQuadrature` first obtains a complete six-branch balance sample. It then
prepares every affected history update in temporary fixed storage. Only a fully
successful update replaces the previous histories. A later quadrature failure
preserves all earlier history and spent work, returns no partial observation and
permanently terminates the wrapper. No rollback changes an already accepted
trajectory state.

## Resources and measured sensitivity

The example reserves 32,010,712 numerical bytes under a 128 MiB cap. The original
probe owners/interpolants account for 22,193,768 bytes; independent balance
observers and their report allowance add 9,779,056 bytes. The quadrature wrapper
and conservative report allowance add 37,888 bytes. Caller manifests, retained
output artifacts and allocator overhead require separate storage budgets.

Ten reports admit sixty child samples, 540 scalar transforms and 977,280
prescribed-force work units, plus 140 complete-manifest clock comparisons.
Quadrature admission bounds 159 comparisons. Attempt accounting conservatively
reserves 180 history updates and 180 integral reads, including allowances for
requests that fail before arithmetic. These work units are not primitive FFT
FLOPs or wall-clock predictions.

The actual example's successive energy-integral changes are approximately
5.49e-12 and 3.43e-13, a factor near sixteen, while every trajectory/interpolant
digest remains unchanged by measurement. The finest measured energy defects
range from about 2.30e-14 to 4.98e-14 in absolute value across the six branches.
These findings show quadrature sensitivity for this short smooth profile. They
do not bound reconstruction error, force error or concentrating trajectories.

Focused tests cover changed physics, skipped probes, invalid geometry, finite
caps, later-child exhaustion, rollback after earlier pending updates, rest samples
with positive-time lookahead owners, and all three complete quadratures. Allocator
instrumentation checks admission without allocation, construction within the
joint cap, and repeated valid/refused observations without heap activity.

P08/P09 remain incomplete. Complete balance/residual arithmetic, reference/force
and regional/location refinements, benchmark/artifact binding and current-grid
concentrating validation remain. No concentrating PDE window is accepted.
