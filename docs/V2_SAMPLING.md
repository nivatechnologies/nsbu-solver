# Exact-v2 nested physical sampling

The `v2_experiment::sampling` library API measures how physical-space
diagnostics change when the sampling lattice changes while the evolved
spectral states stay fixed. It is a consumer of an already admitted `V2Family`;
it does not advance that family, replace any branch, or inject an analytical
reference state.

## Profile and measured quantities

The maintained study uses the actual six-branch V2 family with state grids
`N=4`, `N=8`, and `N=12`. Each accepted family clock is sampled on three
physical lattices:

- `12 × 12 × 12`
- `24 × 24 × 24`
- `48 × 48 × 48`

`SamplingPlan::new` requires strict nesting on every axis. Each next extent
must be larger than, and exactly divisible by, the preceding extent. Thus
`12/24/48` is admitted while the increasing sequence `12/20/48` is refused.
This condition makes every coarse periodic sample location a location on each
finer lattice.

Every report contains velocity, velocity gradient, ordered velocity Hessian,
and vorticity. Each quantity covers the five standard pairs in this order:

1. `N0/N1`
2. `N1/N2`
3. `H0/H1`
4. `H1/H2`
5. `CM/HO`

Pressure is excluded. Its provider and gauge requirements are separate from
the four velocity-derived quantities exposed by this consumer.

## Planning and execution

`SamplingPlan` binds the admitted `FamilyPlan`, the three layouts, four fixed
positive relative floors, the maximum number of attempts, and the joint byte
cap. `SamplingPlan::bounds` reports consumer storage, family-plus-consumer
joint storage, and finite worst-case work. Plan construction performs resource
checks without allocating trajectory, FFT-workspace, or physical-field
buffers.

`SamplingWorkspace::new` allocates the three physical consumers after the plan
has passed admission. Call `next_time` to identify the required accepted family
clock, then call `measure` with that same `V2Family`. Identity, clock, attempt,
and resource failures are structured refusals. `remaining`, `charged_work`,
and `is_terminated` expose the bounded schedule state. A successful measurement
publishes one complete `SamplingSample`; a child failure cannot publish a
partial report.

The maintained, compilable use and refusal controls are in
[`crates/nsbu-benchmarks/tests/v2_sampling.rs`](../crates/nsbu-benchmarks/tests/v2_sampling.rs)
and its
[`actual-family report test`](../crates/nsbu-benchmarks/tests/v2_sampling/actual.rs).
There is no command-line interface for this API.

## Reading a report

`SamplingSample::quantities` returns the four quantity reports. For a quantity,
`pair(pair_index)` returns the three raw `LocalError` values, one per sampling
lattice. RMS uses complete vector or tensor magnitudes:
`sqrt(sum(pointwise squared Euclidean/Frobenius differences) / sample_count)`.
It does not divide by the component count, unlike the offline multiprecision
study's componentwise RMS. Absolute error, relative error, and finer-state
peaks remain separate. `reference_peak` is not an analytical reference value.

`extrema(pair_index)` returns the corresponding location witnesses. Absolute
error, relative error, and reference magnitude each record the producing
layout, the first x-major and z-fast linear index attaining the maximum, its
three-dimensional index, and its value. Complete admitted grids produce a
`Measured` witness. `NoSamples` exists so an empty helper input cannot be
misreported as a zero maximum.

`changes(pair_index)` returns absolute changes in each statistic from the first
lattice to the second and from the second to the third. RMS remains separate
from every peak statistic, and each quantity keeps its own relative floor.
These differences do not imply a rate, monotonicity, or convergence order.

## Resources and interpretation

The maintained profile reserves 34,644,696 bytes jointly for the family and
all three simultaneous consumers. The allocation audit observed 29,520,704
bytes during construction, zero admission allocations, and zero steady-state
allocations. Process RSS includes the test harness and runtime and is not the
reservation. Extrema scans and retained report storage are included in the
admitted work and byte accounting.

These are finite-grid empirical diagnostics. A sampled maximum is not a
continuous-domain supremum, and agreement across `12/24/48` alone does not
establish convergence or qualify a tracking window. Raw measurements,
allocation results, source hashes, and limitations are retained in
[`evidence/p09/v2-nested-physical-sampling`](../evidence/p09/v2-nested-physical-sampling/README.md).
That evidence reports both focused coverage scopes: library source passed its
focused threshold, while source plus integration assertions measured 78.26%
branch coverage. The complete maintained-source coverage gate remains pending
the combined hosted run.
