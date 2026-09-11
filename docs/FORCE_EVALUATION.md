# Exact-v2 force evaluation with bounded reusable algebra

`V2Force` evaluates the unchanged `similarity-mms-v2` prescribed force on an
explicit sampling grid and returns normalized retained Fourier coefficients.
Its implicit root depends on axial coordinate z and physical time, so every
point in a sampled axial plane can reuse that root and its degree-four jet.
The provider prepares one checked entry per active plane on each call. It still
assembles the complete pointwise force independently at every spatial sample.
Compile-time derivative tables also remove repeated coefficient-index searches
from the degree-four jet algebra.

This is an evaluation optimization. It does not modify the case definition,
remove force structure, project the physical force before sampling, initialize
velocity from a reference, or advance a trajectory. Grid and arithmetic
qualification remain separate studies.

## Optional reduced arithmetic provider

`provider::reduced::ReducedV2Force` implements the same bounded
`PrescribedForce` contract using the reduced `(w,z,t)` degree-three evaluator.
It is explicitly selected and remains separate from `V2Force`: existing `Run`,
`ForceSettings`, CLI and checkpoint paths retain the original provider. Its
different binary64 operation order requires separate word comparisons and
artifact identity. It does not evolve a trajectory.

The provider is exercised separately by a bounded N=4 trajectory check, which
owns independent CM and HO runs from rest and compares reduced and original
provider paths with 80/120-digit fixtures. This does not change default provider
selection.

Call `ReducedV2Force::preflight(domain, sampled)` before construction. The
returned `ForceLimits` covers FFT plans and scratch, three physical sample
buffers, one reusable transform spectrum, one axial-root slot per sampled
z-plane and provider metadata. The caller reserves the three retained output
spectra separately; the public comparison profile includes both output sets.
`ReducedV2Force::new` refuses a smaller cap before allocating. Each request has
the conservative bound `129 * sampled.real_len()` work units and three scalar
transforms; these units are implementation work, not FLOPs or elapsed time.

For each exact `TickClock`, one checked root cache is built per active axial
plane and reused across that plane's points. Its key binds the exact z word,
elapsed and remaining time words, and both conversion-error estimates. The cache
is rebuilt per request, supports nonmonotone and repeated clocks, and rejects a
mismatched root rather than falling back to an uncharged solve. Rest and flat
support retain the exact zero branch.

## Public bounded profile

```sh
cargo run --release -p nsbu-benchmarks --example force_cache_profile -- --dry-run
cargo run --release -p nsbu-benchmarks --example force_cache_profile
cargo test -p nsbu-benchmarks --test axial_force_cache --test provider
```

The profile uses retained grids N=8/12/16 and sampling grids M=12/18/24 on the
unit periodic cube, viscosity one. Its exact tick quantum is 2^-10, target tick
8, and request sequence 1/4/2/1 deliberately moves backward and repeats a time.
Each grid prints the admitted provider storage/work/FFT limits and combined
reservation before construction. The 2 MiB numerical cap includes three output
spectra and a 64 KiB metadata/output allowance. Timing excludes output formatting
and hashing. Syntax errors and resource refusals return a failing process status.

The argument-free command prints all twelve complete coefficient SHA-256 hashes,
actual safeguarded root iteration counts and declared provider storage. A hash
is a reproducibility check, not a bound on force error. This program performs no
PDE integration and accepts no concentrating window.

## Cache identity and lifetime

An entry contains the unchanged scalar root report, implicit jet and full formal
residual jet. The key binds the periodic axial coordinate, separately converted
elapsed and remaining times, and both conversion-error estimates. The provider
rebuilds the entire plane cache for every exact-clock request. Missing, wrong-plane
or mismatched computational inputs are refused for an active point; there is no
fallback to an uncharged root solve.

The exact `TickClock` remains the integrator's time identity. The private cache key
identifies binary64 inputs to this computation and does not replace that clock.
In particular, near the target two exact clocks can round to the same elapsed
value while their remaining values differ. A dedicated test demonstrates this
collision and rejects reuse across it. Exact rest and points outside the smooth
support retain the original zero branch and require no root.

`fields::evaluate` retains the uncached public pointwise implementation for
comparison and reference diagnostics. Cache entries contain no velocity state,
accepted history, force artifact provenance or continuation authority. They are
derived scratch and cannot substitute for checkpoint lineage.

## Precomputed derivative indices

A degree-four polynomial in four variables has 70 coefficients; its derivative
has only 35 possibly nonzero coefficients. For each axis, an immutable table
records those output/source indices and their integer multiplier. Evaluation
performs the same 35 floating multiplications, in the same order, as the original
search path. Unavailable top-degree coefficients remain positive zero. The table
changes no polynomial order, convolution, formal Newton step or truncation rule.

An independent linear-search oracle checks every monomial and a dense mixed
coefficient vector, including signed zeros and very small/large finite values.
Tests also retain structured overflow and invalid-axis refusals. The original
public derivative lookup and the independent Python scalar/jet fixtures remain
separate checks. Table storage is immutable program data, not per-call heap state.

## Storage, work and allocation

Preflight adds M_z times `size_of::<Option<AxialRoot>>()` to the owned reservation.
There are twelve provider heap allocations, including the new fixed-length plane
buffer. The reservation includes their header allowance; an insufficient cap is
refused before any allocation. Allocation instrumentation checks the constructor
and repeated valid/refused evaluations. The force call itself allocates nothing.
Fixed-size jet temporaries use stack storage, as in the original evaluator.

The conservative work maximum remains 129 times the number of sampled points:
one bounded field assembly and up to 128 safeguarded scalar iterations per point.
Actual reported work is the number of sampled points plus the scalar iterations
performed once per active axial plane. All three FFTs remain separately counted.
These units bound this implementation's calls; they are not primitive operations
or a wall-clock prediction. Prior resource plans and work measurements describe
their original implementation profile; do not overwrite historical evidence with
new counts or reuse an obsolete reservation.

## Verification and performance limits

Word-for-word comparisons cover velocity, raw pressure, force, every force
gradient and cancellation diagnostic, divergence, formal residual coefficients
and scalar root report. They include rest, axis, interior, collar, exterior,
periodic coordinates and nonmonotone times. A separate anisotropic M=6/8/12 grid
test rebuilds all force samples through the uncached point path, independently
transforms them and checks every retained coefficient word. The independent
Python direct-DFT force fixture also remains a required provider test.

Separate initial profiles preserved every coefficient hash: axial reuse alone
reduced measured force-call time by about 20%, while derivative tables alone
roughly halved it. The first combined profile was 2.65 to 3.10 times as fast as
the original baseline over the twelve tested calls.
At N=16/M=24, the first request used 98 scalar iterations instead of 19,426;
provider reservation increased from 575,128 to 604,408 bytes. These are small-grid
measurements, with ordinary host scheduling noise. Initial isolated measurements and a
separate interleaved baseline/combined profile are retained with execution evidence.
The three interleaved pairs preserve all 36 complete coefficient hashes; their
median speed ratio is 3.046 (individual ratios 2.576 to 3.687). They do not predict a large-grid run time or qualify binary64 accuracy.

Remaining costs include full degree-four pointwise jet assembly and serial grid
sampling. A smaller force-only algebra or parallel backend would need its own
bounded resource design, independent numerical comparisons and transaction tests.
Neither is supplied by the axial cache. P08/P09 and concentrating PDE validation
remain incomplete.

The separate [persistent parallel backend](PARALLEL_FORCE.md) retains the serial
provider for comparison while moving physical sample evaluation to explicitly
reserved workers. Its concurrency and allocation checks have their own evidence.

The reduced force-only profile is
`cargo run --release -p nsbu-benchmarks --example reduced_provider_profile`;
append `-- --dry-run` for admission only. It uses retained N=16, sampled M=24,
quantum `2^-10`, target `8`, and request clocks `[1,4,2,1]`. It prints hashes
and arithmetic differences against the original provider. The
[recorded profile](../evidence/p09/reduced-provider/README.md) measures a median
6.48 speed ratio for complete provider requests on one shared host, with maximum
scaled coefficient difference 2.35e-14. This does not measure trajectory speed or
qualify force-grid resolution.

## Verified trajectory profile

```sh
cargo test -p nsbu-benchmarks --test reduced_trajectory -- --nocapture
```

The profile uses N=4/M=4, exponent `-20`, target `8192`, step 128, 32 commits,
and endpoint 4096 (`t = 1/256`). CM and HO are evolved independently from rest
with both providers after full joint preflight. The reduced trajectory is
checked against the original-provider trajectory at a `5e-12` scaled component
threshold after every commit; all three components at the 18 explicitly listed
strict-band endpoint modes (excluding Nyquist slots) pass independent
80/120-digit trajectory fixtures at `5e-13`. Maximum CM and HO pair differences are
`2.915242e-16` and `4.5380187e-16`; endpoint maxima are `4.8169699e-16`
(original provider) and `2.9661178e-16` (reduced provider).

The [retained evidence](../evidence/p09/reduced-trajectories/README.md) records
arithmetic and bounded from-rest trajectory checks for this N=4 profile.
They provide no trajectory timing, provider speed, force-grid convergence or PDE
qualification result. Broader arithmetic identity and runtime-integration work
remains open.
