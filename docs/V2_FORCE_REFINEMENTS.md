# Exact-v2 force-sampling trajectory refinements

`nsbu_benchmarks::v2_force_experiment` evolves three independently owned
`similarity-mms-v2` trajectories from exact rest. The retained velocity grid N,
macro step, integration method, endpoint, physical domain, viscosity, tolerances,
advective guard and worker selection are fixed. Only the prescribed-force sample
grid changes through three strictly increasing nested values M0, M1 and M2.

This family measures force-sampling sensitivity separately from retained velocity
resolution. It reports no force-sufficiency, convergence or accepted-window status.
It does not replace the current force arithmetic, reference, physical-observable,
transfer or provenance studies.

The separate [sampled physical consumer](V2_FORCE_PHYSICAL.md) evaluates
velocity, ordered gradient, ordered Hessian and vorticity RMS and peaks directly
from these same immutable states. It does not reinterpret the spectral H1 value.

## Admission and identity

`ForceFamilyPlan::new` validates the complete exact-clock manifest and every
branch before allocation. Cubic force grids must be strictly increasing, and
each finer size must be divisible by its predecessor so the physical sample
lattices are nested. Every manifest clock aligns with the fixed macro step and
the final clock equals the declared endpoint.

The aggregate bound includes three complete `v2_run::Run` reservations, total
attempts, integration calls, force and observer work, scalar transforms, both
full-band spectral comparisons at every sample, fixed family metadata and the
streaming identity traversal. The caller must separately reserve retained output
or serialization buffers. A cap smaller than the aggregate reservation is
refused before constructing a numerical owner.

Identity version 1 starts with `NSBUV2FORCEFAM01` and binds the reviewed case
hash; fixed N; all three M values; worker count; step and method; endpoint; all
tolerance and guard binary64 words; and every exact clock exponent, target,
elapsed and remaining word. Integers use little-endian u128 encoding except the
method's little-endian u64 tag and clock exponent's little-endian i32. The hash
is streamed without allocating a serialization buffer.

## Evolution, comparisons and failures

Each branch owns its provider, state, candidate, integrator scratch, balance
observer, history and attempt-work ledger. The API exposes state only through
the existing read-only `v2_run::Run` accessors; there is no analytical assignment
or reset path. A sample is published only after all branches reach the same exact
accepted clock.

Pair order is M0/M1 followed by M1/M2. Both use the reusable full-band
`ComparisonPlan` on the fixed retained domain. `full.l2` is the complete retained
velocity difference. `full.h1` contains that velocity term and every first
spectral derivative. Since N is identical, `common` equals `full` and the
newly-resolved retained-velocity contribution is exactly zero; this does not
discard any retained mode. Mean, curl and divergence differences remain present
in each `BandComparison`.

Any child rejection, refusal or numerical error terminates the family. Earlier
child commits and all charged failed work remain inspectable. There is no family
rollback, analytical reset, hidden retry or fresh attempt allowance after a
terminal result.

## Bounded post-startup profile

Run the documented profile with:

```sh
cargo run --release -p nsbu-benchmarks --example v2_force_refinement
```

It uses N=4, nested M=[4,8,16], CM, a 64-tick step, quantum 2^-20 and endpoint
512, or physical time 1/2048. The admitted family owns 24 total attempts, 288 RHS
calls and 96,430,080 force/observer work units within 2,503,096 reserved bytes.
At the endpoint, its observed full-band values are:

| Force-grid pair | Velocity L2 | Derivative-sensitive H1 |
| --- | ---: | ---: |
| M4/M8 | 2.4424439651500145e-2 | 2.2572582941869e-1 |
| M8/M16 | 5.726571685966544e-3 | 5.11740607059927e-2 |

These finite values are a force-sampling sensitivity measurement on a coarse,
short post-startup trajectory. Their ordering is not treated as monotonicity,
a fitted rate, adequate force resolution or evidence for the first concentrating
endpoint. Accepted concentrating windows remain zero.

## Verification

```sh
cargo test -p nsbu-benchmarks --test v2_force_family
cargo test -p nsbu-benchmarks --test v2_force_family_allocation
cargo test -p nsbu-benchmarks --example v2_force_refinement
```

The independent oracle sums signed full-complex Fourier modes using separate
indexing, conjugation, wave numbers and norm accumulation. It checks both actual
endpoint pairs. An ownership/self-comparison negative control demonstrates the
false zero obtained by reusing one state while both actual pairs remain measurably
nonzero; separate solver tests cover force aliasing. Additional tests cover exact identity words,
manifest and nesting refusal, aggregate-cap refusal, private ownership, terminal
failure state, attempt retention, construction bounds and allocation-free
admission/evolution.
