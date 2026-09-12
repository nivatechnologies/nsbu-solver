# Exact-v2 nominal core/annulus coverage

`nsbu_benchmarks::v2_experiment::coverage` attaches bounded empirical nominal
coverage fractions to an actual accepted `RegionalTrackingSample`. It reuses
[`regions::CoveragePlan`](../crates/nsbu-benchmarks/src/regions/coverage.rs):
three strictly nested, divisible even panel settings each evaluate their own
coarse/fine composite Simpson pair. The consumer reports every raw fraction,
within-setting refinement change, finer panel count, and geometry-evaluation
count for the fixed nominal Core (`0 <= X <= 1/2`) and Annulus
(`1/2 < X <= 8`) intersections with the mathematical `c_x = 1` set on
`|eta| <= 1/2`.

The plan binds an exact accepted clock, V2 family identity, regional-tracking
identity, all six branch labels, all four quantity labels, and the complete
actual sample-grid count. Its sampled Core/Annulus metadata is separate from
the nominal fractions. `CoverageStatus::Nonempty` is geometric and empirical;
a sampled class may be `NoSamples` without becoming geometrically empty.
Conversely, a nonzero node count does not establish nominal volume.

`CoverageFamilyPlan` reserves the six-run family plus this consumer only. The
caller retains and budgets the regional tracking workspace separately. A
consumer attempt charges both nominal regions and all three plans before
validation; a foreign, stale, invalid, exhausted, or failed call publishes no
new report. The workspace retains only its last complete report and borrows all
actual state read-only.

The focused startup test uses panels 256/512/1024 and checks the annulus words
against the independent high-precision fixtures generated from
[`reference/regions.py`](../reference/regions.py). It also checks state
immutability, identity/clock rejection, nested-panel admission and cap refusal.

This is empirical core/annulus geometry only. It is not a volume enclosure,
collar-volume report, global qualification, full observable inventory, generic
measurement review, or PDE-window decision. The active
[implementation plan](../IMPLEMENTATION_PLAN.md) still requires cutoff-collar
coverage and every other channel/provenance requirement.
