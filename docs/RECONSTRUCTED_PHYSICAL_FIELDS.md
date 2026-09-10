# Physical diagnostics at reconstructed probe times

`ProbeDiagnostics` computes complete velocity, gradient, Hessian, vorticity,
physical pressure and pressure-gradient refinements from the six independently
evolved [accepted-history probes](RECONSTRUCTED_PROBES.md). The physical time is
the requested probe clock. A branch's later accepted lookahead state supplies
its interpolation nodes; it does not replace the reconstructed diagnostic field.

The consumer borrows `ProbeFamily` immutably. It cannot assign a reference, modify
an integrated state or change an interpolant. It binds the exact trajectory
settings and complete probe manifest before numerical work. Arbitrary supplied
field views cannot be promoted to this privately constructed owner provenance.

## Run the public example

```sh
cargo run --release -p nsbu-benchmarks --example smooth_probe_diagnostics -- --dry-run
cargo run --release -p nsbu-benchmarks --example smooth_probe_diagnostics
```

The CyclicSine profile uses retained grids N=4/8/12, macro ticks H=64/32/16,
quantum 2^-16, endpoint tick 128, physical probes 0/7/128 and sample grid M=24.
CM and HO evolve independently from rest. The six fixed relative floors are
1e-8/1e-7/1e-6/1e-7/1e-8/1e-7 in the quantity order above. These are diagnostic
normalizers, not established concentrating arithmetic floors or window tolerances.

Every probe produces thirty complete quantity/pair findings: N0/N1, N1/N2,
H0/H1, H1/H2 and CM/HO for each quantity. Every retained fine mode and every
ordered derivative component contributes. RMS values divide by the number of
physical sample points, with complete vector/tensor magnitudes. Primary errors
are unaligned; no recentering, phase shift or common-band crop is applied.

Output retains the actual physical probe clock, all accepted-node triples,
current integrated-state clocks and every original sampled RMS/peak/relative/
reference statistic. `ProbePhysicalSample::reconstruction()` also preserves the
complete Fourier value/time-derivative comparisons from that exact probe.

## Pressure and time binding

Each pressure is independently assembled from the reconstructed velocity and a
fresh **unprojected prescribed force at the physical probe time**. Conservative
products retain the full doubled velocity band. The diagnostic does not reuse a
stage RHS, interpolate stage pressure or substitute analytical pressure.

All pairs use the same complete finest doubled domain. A coarser reconstructed
velocity is transferred without truncating any original mode before conservative
products are formed. Reported physical pressure has one global mean-zero gauge;
regional refitting and fitted offsets are absent. The common physical sample
grid must retain this complete pressure band.

At probe tick 0 the actual lookahead states already have positive times, but the
reconstructed rest fields and physical-pressure statistics remain zero. This is
an explicit test of using the probe field and freshly timed force. At tick 7,
sampling the later actual states yields a substantially different velocity
comparison; the test requires the report to retain the reconstructed result.

## Resource and failure contracts

The public example jointly admits 26,586,600 numerical bytes under a 128 MiB cap:
22,193,768 bytes for all six owners and interpolation scratch, plus 4,392,832 bytes
for the complete physical/pressure consumers and conservative report allowance.
Caller-owned manifests, retained output and allocator overhead remain separate.

The three probes charge 1,350 physical scalar transforms and 390 pressure
transforms, with 67,392 prescribed-force work units. Their additional weighted
visits are 91,111,872 and 34,977,792 respectively; these are not FFT-internal FLOPs
or wall-clock predictions. Original integration and accepted-history work retain
their independently admitted budgets. The existing physical and pressure kernels
are reused through private borrowed-probe entry points; their accepted-time
schedule counters are not used by the probe consumer.

Each request consumes a finite aggregate attempt. A malformed or skipped probe
is refused before either child does numerical work; it may be corrected if an
attempt remains. Child ledgers charge complete numerical attempts as they run.
Any child failure permanently terminates the aggregate, preserving spent work
and earlier legal trajectory commits, with no partial complete report. Failure
after physical sampling but before pressure publication has a dedicated test.

Allocation instrumentation verifies no heap activity during admission, complete
construction within the joint reservation, and no allocations/deallocations/
reallocations during repeated successful or refused measurements. The plan
refuses insufficient storage, invalid floors, insufficient pressure bandwidth,
work overflow and inadequate attempt allowances before owner construction.

## Scope of the evidence

Tests cover complete probe provenance, all thirty findings, exact rest zero,
nonzero off-stage differences, unchanged state and interpolant digests, resource
refusal, changed policy/manifests, skipped probes and terminal child failure.
Existing independent field, tensor, pressure-band and direct-DFT arithmetic
fixtures continue to verify the reused numerical kernels separately.

This adds physical measurements at accepted-history probe times for the smooth
case. It does not quantify reconstruction error across the complete window,
provide continuous-time enclosures, complete reference/force/arithmetic/regional/
location channels, authenticate external artifacts or qualify a concentrating
trajectory. P08/P09 remain incomplete; accepted concentrating PDE windows are zero.
