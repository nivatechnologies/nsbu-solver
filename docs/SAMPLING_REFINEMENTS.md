# Physical sampling refinements

`nsbu_benchmarks::smooth_experiment::sampling` measures how physical diagnostic
statistics change when their sample lattice is refined. It observes the same
six independently evolved CyclicSine trajectories at the same accepted clock.
It does not change their retained grids, time steps, force, integrated state or
physical alignment.

The consumer covers velocity, the complete gradient and Hessian, vorticity,
mean-zero physical pressure and its complete gradient. Each quantity includes
two spatial pairs, two temporal pairs and CM/HO. Three sample grids therefore
produce **90 complete field comparisons per clock**. All original statistics
remain available; no coarse sample or failed comparison is discarded.

This is an implemented smooth diagnostic channel. It does not establish
concentrating convergence, a continuous spatial supremum or a supported
roundoff floor. P08/P09 and the concentrating window verifier remain incomplete.

## Run the public example

From the repository root with the pinned Rust toolchain:

```sh
cargo run --release -p nsbu-benchmarks --example smooth_sampling -- --dry-run
cargo run --release -p nsbu-benchmarks --example smooth_sampling
```

The fixed profile uses retained grids N=4/8/12, macro steps 64/32/16 at quantum
2^-16, and endpoint tick 128 (time 1/512). Each branch starts from rest and
commits its own attempts. Physical sample grids are M=24/32/48; the same
positive floors apply to every sample resolution and comparison pair:

| Quantity | Ordered components | Fixed relative denominator floor |
| --- | ---: | ---: |
| Velocity | 3 | 1e-8 |
| Gradient | 9 | 1e-7 |
| Hessian | 27 | 1e-6 |
| Vorticity | 3 | 1e-7 |
| Pressure | 1 | 1e-8 |
| Pressure gradient | 3 | 1e-7 |

These are declared smooth diagnostic settings, not a frozen concentrating
acceptance protocol. The dry-run admits the full joint reservation and prints
all sample grids, floors, finite work and the 128 MiB cap before allocating any
numerical owner. The argument-free run prints the same preflight, then measures
ticks 0/64/128. Unknown or extra arguments fail.

## Read the measurements

Output pair numbers are fixed:

| Pair | Independently evolved states |
| ---: | --- |
| 0 | Coarsest/middle retained grid, finest macro step, CM |
| 1 | Middle/finest retained grid, finest macro step, CM |
| 2 | Coarsest/middle macro step on the finest retained grid, CM |
| 3 | Middle/finest macro step on the finest retained grid, CM |
| 4 | CM/HO on the finest retained grid and macro step |

For each pair, `SamplingQuantity::pair` returns all three original `LocalError`
records. They contain RMS error, sampled absolute and relative error peaks,
reference peak, physical sample count, complete component count and fixed floor.
The complete field difference is formed before reduction. RMS divides by sample
count, never by tensor entries.

`changes` reports the absolute change in each statistic between M0/M1 and M1/M2.
These are **changes in diagnostic sampling**, not replacements for the underlying
field differences. Matching RMS values cannot qualify a sampled maximum. A
phase-shifted cosine test has the same exact RMS on three sample grids while
their captured peaks differ, demonstrating this distinction.

The sample grids increase strictly along every axis. They need not nest; no
monotone peak or fitted convergence rate is assumed. Every grid must retain the
full pressure band and satisfy the current radix-2/3 FFT backend's admission
rules. Coordinates are unshifted `[0,L)` with z varying fastest. There is no
recentering, phase rotation or alignment. Full pressure is reconstructed from
each actual velocity and independently prescribed unprojected force on twice
the finest retained grid; the exact smooth pressure zero is never substituted.

Tiny sampling changes at exact rest or near the binary64 cancellation level do
not prove that an error floor is supported. A separate numerical policy must
review sampling changes alongside all remaining channels, including physical
reduction arithmetic and reference accuracy.

## Ownership, work and failures

`SamplingPlan::new` borrows the existing `FamilyPlan` and admits all three physical
and three pressure consumers together with the six trajectory owners. It adds
fixed metadata and conservative stack/report scratch. Other consumers, retained
output files, caller buffers and allocator overhead require their own budgets.
Checked arithmetic refuses unsupported grids, inadequate caps and work overflow
before allocation.

One complete attempted report charges **1,740 scalar transforms**, the three
independent force-provider allowances, and explicitly counted conservative
weighted visits. Weighted visits exclude FFT internals and are not FLOPs or a
wall-clock estimate. Failed requests retain these worst-case charges; no hidden
retry obtains a fresh allowance.

The family is borrowed read-only. The consumer first checks the exact accepted
clock, complete manifest and unchanged numerical policy words. A malformed
request spends an attempt, keeps its schedule position and can be corrected
before any child advances. If a numerical child fails after work begins, the
aggregate consumer becomes permanently terminated: it publishes no partial
record and cannot restart with inconsistent child histories. Earlier actual
trajectory commits remain intact.

The complete result is privately constructed only after all 90 comparisons
succeed. It records its actual clock and three sample lattices. It cannot issue
an accepted PDE window or authenticate external scientific artifacts.

This consumer observes synchronized accepted clocks supported by `FamilyPlan`;
its tested times must align with the coarsest macro step. It does not interpolate
a finer time manifest. Off-stage family sampling needs separately admitted
reconstruction from each branch's own accepted history, with its reconstruction
errors retained. The existing last-two-interval history cannot be used after the
fact to reconstruct arbitrary earlier probes across an entire window.

## Source and verification

- [Plan](../crates/nsbu-benchmarks/src/smooth_experiment/sampling/plan.rs): complete
  immutable resource and work admission.
- [Consumer](../crates/nsbu-benchmarks/src/smooth_experiment/sampling/mod.rs): actual
  state binding, finite attempts and complete publication.
- [Reports](../crates/nsbu-benchmarks/src/smooth_experiment/sampling/report.rs):
  retained measurements and explicitly labeled sampling changes.

```sh
cargo test -p nsbu-benchmarks --lib sampling
cargo test -p nsbu-benchmarks --test sampling_family --test sampling_allocation
cargo test -p nsbu-benchmarks --example smooth_sampling
```

Tests cover unchanged full-state digests, complete tensors/pairs/grids, exact
clocks, malformed/exhausted requests, aggregate-cap refusal, invalid floors and
unsupported grids, the missed-peak control, and a desynchronized child that must
terminate the consumer. The isolated allocator executable measures admission,
construction, accepted evolution, complete measurement and refusal paths.
