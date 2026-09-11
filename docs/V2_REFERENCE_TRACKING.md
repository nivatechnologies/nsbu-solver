# Exact-v2 analytical trajectory tracking

`nsbu_benchmarks::v2_experiment::reference` measures six independently evolved
exact-v2 family states against the analytical manufactured velocity. It covers
velocity, all nine ordered first derivatives, all 27 ordered Hessian entries and
the physical three-component curl. Pressure is outside this consumer.

The consumer borrows an already admitted `FamilyPlan` and observes `V2Family`
through read-only state access. It cannot assign, reset or replace numerical
coefficients. Every branch must have `InternalFromRest` lineage in the documented
profile, and all branches must reach the exact next manifest clock before a report
can be produced.

## Sampling and reference reuse

Actual fields use `DerivativeWorkspace` to zero-pad and inverse-transform every
complete retained strict band on one independently declared sample lattice.
Velocity uses three transforms, gradient nine, ordered Hessian 27 and curl six,
for 45 transforms per branch and 270 per complete six-branch report. Coordinates
are the unshifted periodic grid `[0,1)^3`, with z varying fastest.

The analytical `fields::reference::evaluate` call supplies all four tensors at
one point. Each point is evaluated exactly once per report clock and cached for
all six branches and all tensor entries. The cache is diagnostic storage; it is
never converted into spectral state. Error and reference magnitudes accumulate
components through checked `hypot`, preserving small nonzero terms without
intermediate square overflow.

Each quantity reports global sampled RMS error, sampled absolute and relative
peaks, analytical reference peak, complete component count, sample count and a
fixed positive relative denominator floor. RMS divides by physical sample count,
never tensor entry count. These are sampled values rather than continuous
supremum or regional bounds.

## Admission and failure behavior

`ReferenceTrackingPlan::new` jointly admits the existing six-run family, three
derivative workspaces for its N4/N8/N12 retained grids, four physical vectors,
the complete analytical cache, report scratch and conservative allocator
allowances. It also declares finite attempts, reference evaluations, root
iterations, scalar transforms and weighted coefficient/sample visits. The root
allowance charges 128 iterations at every requested point, including flat points,
matching the existing evaluator's finite implicit-root bound.

Every call charges the full worst-case attempt before validating the borrowed
family. Foreign identity, stale clock and exhausted-attempt refusals publish
nothing and leave the required schedule position unchanged. If analytical or
numerical computation fails after validation begins, the consumer terminates and
cannot obtain a fresh allowance. A successful schedule advance occurs only after
all 24 branch/quantity findings exist. Integrated state, clock, history, family
identity and origin remain unchanged.

## Bounded profile and interpretation

Run:

```sh
cargo run --release -p nsbu-benchmarks --example v2_reference_tracking
```

The profile uses the existing N=[4,8,12], steps=[64,32,16], fixed M=12 CM family,
sample grid 12, quantum 2^-20 and endpoint 128, or physical time 1/8192. Three
reports admit 5,184 reference evaluations, 663,552 root iterations, 810 scalar
transforms and 4,920,528 weighted visits. At the endpoint, the N12/h16 CM branch
has sampled RMS errors:

| Quantity | RMS tracking error |
| --- | ---: |
| Velocity | 1.8873998839615518e-7 |
| Gradient | 9.142791231564898e-6 |
| Ordered Hessian | 9.325614204173285e-4 |
| Vorticity | 9.0193622044277e-6 |

The exact-rest report is zero because both the independent trajectory and
analytical field are exactly zero there. The nonzero endpoint values are retained
as coarse global sampled tracking errors. No monotonicity, fitted convergence,
reference sufficiency or accepted-window result follows from them.

The Rust analytical evaluator uses reviewed binary64 jet/root arithmetic. An
existing 120-digit Python fixture cross-check is retained, but this increment
does not resolve current-grid arithmetic error. Regional coverage, pressure and
its gauge, force/transfer sufficiency, artifact provenance and the first
concentrating endpoint remain separate qualification work. Accepted concentrating
windows remain zero.

## Focused verification

```sh
cargo test -p nsbu-benchmarks --lib v2_experiment::reference::tests
cargo test -p nsbu-benchmarks --test v2_reference_tracking
cargo test -p nsbu-benchmarks --test v2_reference_tracking_allocation
```

The independent oracle reconstructs actual values and derivatives through its
own signed full-complex Fourier sums, physical phases and wave numbers, then
checks all four statistics for every branch and quantity. The reference path is
cross-checked against the existing 120-digit tensor fixture. Tests also cover
tiny/large magnitude arithmetic, exact rest, private origins, foreign and stale
families, terminal child state, attempt exhaustion, invalid floors/sample grids,
work overflow, cap refusal, state integrity and allocation-free execution.
