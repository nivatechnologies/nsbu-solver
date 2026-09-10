# Streaming accepted-history probes

`smooth_experiment::probes::ProbeFamily` observes six independent smooth
trajectories at exact times between their accepted steps. It samples each branch
while its bounded accepted history still covers the requested time. This makes
early off-stage observations available alongside later ones, without retaining
every historical state or resetting any trajectory.

The owner evolves all branches from rest. It has no analytical-reference
assignment interface and does not import checkpoints. Interpolated fields live
in separate diagnostic scratch; they never replace an integrated `SpectralState`.
The owner provides complete Fourier value/time-derivative comparisons. The
[physical consumer](RECONSTRUCTED_PHYSICAL_FIELDS.md) adds complete tensors and
pressure with a fresh prescribed force at the probe time. Full reference/force
refinement, residual integration and window qualification remain.

## Execute the public workflow

```sh
cargo run --release -p nsbu-benchmarks --example smooth_probes -- --dry-run
cargo run --release -p nsbu-benchmarks --example smooth_probes
```

The fixed CyclicSine profile uses N=4/8/12, macro ticks 64/32/16, quantum 2^-16,
and endpoint tick 128 (time 1/512). Both CM and HO evolve independently. Its exact
tested-time manifests are:

```text
coarse: 0, 64, 128
middle: 0, 31, 64, 95, 128
fine:   0, 7, 31, 63, 64, 95, 127, 128
```

The example verifies strict nesting and evaluates the fine manifest. This records
more observation times on unchanged trajectories; it does not refine their time
steps or claim error control between probes. The dry-run checks the complete joint
reservation and every reconstruction geometry before allocating numerical owners.
It prints the finite work, lookahead convention and 128 MiB cap. Unknown or extra
arguments fail.

## Physical probe time and actual state time

The quintic reconstruction uses three **actual accepted macro endpoints** and
their independently computed physical-time derivatives. For macro interval H,
probe tick t uses node indices:

```text
j = max(2, ceil(t/H))
nodes = [(j-2)H, (j-1)H, jH]
```

This needs two initial macro steps even for an early probe at t=0. An endpoint
shorter than that lookahead is refused. Every node must remain within the admitted
window, and all exact-clock/duration checks must pass. There is no extrapolation.
The first probe therefore becomes available after the necessary future nodes
have been independently integrated; it is an offline diagnostic observation.

For probe tick 7, the finest-step branches have actual state tick 32 and nodes
0/16/32; the coarsest temporal branch has state tick 128 and nodes 0/64/128.
All six reconstructed fields still refer to the same **physical probe tick 7**.
Output preserves both clocks and all node triples. A reconstructed node value is
not silently relabeled as the owner's current integrated state.

As probes increase, each owner advances only when more accepted nodes are needed.
The probe is sampled before later steps overwrite its history. At the final
endpoint, the finest branch's ring cannot reconstruct tick 7 anymore, but the
earlier measurement has already been produced with its exact original provenance.

## Reports and read-only fields

`ProbeFamily::advance()` returns one privately constructed `ProbeSample` only
after all six reconstructions and all ten complete comparisons succeed. Five
comparisons measure velocity and five measure its reconstructed physical-time
derivative, in this order:

| Slot | Comparison |
| ---: | --- |
| 0 | Coarsest/middle retained grid, finest macro step, CM |
| 1 | Middle/finest retained grid, finest macro step, CM |
| 2 | Coarsest/middle macro step, finest retained grid, CM |
| 3 | Middle/finest macro step, finest retained grid, CM |
| 4 | CM/HO, finest retained grid and macro step |

Each `BandComparison` retains full fine-band L2/H1/vorticity/divergence values,
plus separately reported common and newly resolved bands. No phase shift,
recentering or common-band crop is applied to the primary comparison.

`fields(branch)` exposes complete reconstructed value and derivative coefficients
with their domain, physical probe clock and accepted-node origin. The immutable
view borrows diagnostic scratch, so the owner cannot advance while that view is
live. It is not a checkpoint or an integrated state. Out-of-range indices return
`None`. Before the first complete probe, or after a failed next probe, no fields
are exposed. After normal completion, the last successful views remain readable.

The current interface does not accept arbitrary external measurements or perform
lineage authentication. Its from-rest origin follows from privately constructed
owners. External artifact bindings must preserve that distinction.

## Admission and failure behavior

`ProbePlan` borrows the original `FamilyPlan` and an exact `TestedTimes` manifest
on the same window. It admits all six complete owners, six full-band velocity
and derivative interpolants, fixed metadata and conservative report scratch.
Caller-owned time manifests and retained output artifacts, allocator overhead,
and additional diagnostic consumers have separate budgets.

Every admitted physical time receives six bounded Hermite geometry checks.
Interpolation/comparison visits are counted separately from each trajectory's
original integration, provider and accepted-history work. The report counter is
finite, and checked arithmetic refuses cap/size/work overflow before allocation.
Weighted visits describe conservative numerical traversal, not FLOPs or seconds.

On any branch rejection, refusal or reconstruction failure, the owner clears
current diagnostic views and becomes permanently terminated. It retains every
earlier legal state commit and the spent work. No partial aggregate escapes, no
earlier probe is skipped, and no implicit retry obtains a fresh allowance.
Normal exhaustion returns `None`; it does not represent PDE qualification.

## Verification and remaining work

```sh
cargo test -p nsbu-benchmarks --test probe_family
cargo test -p nsbu-benchmarks --test allocation
cargo test -p nsbu-benchmarks --example smooth_probes
```

Tests retain early probes, require exact accepted-node origins, compare endpoint
interpolants with actual states, and compare every final state word with a
separate independently run family. They refuse changed clocks/windows,
insufficient initial lookahead, inadequate caps, work overflow and rejected
branches. Isolated allocator instrumentation covers admission, complete owner
construction, repeated probes and normal exhaustion. Existing independent
polynomial/Hermite fixtures verify the interpolation mathematics separately.

This schedule addresses the bounded-history timing gap; the physical consumer
adds complete sampled tensors and pressure. Full regional/reference tracking,
all-channel arithmetic and force studies,
reconstruction-error qualification, frozen benchmark semantics, external lineage
binding and concentrating convergence remain incomplete. Accepted concentrating PDE windows remain
zero; sampled interpolation is not a continuous-window enclosure.
