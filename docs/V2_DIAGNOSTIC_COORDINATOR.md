# Exact-v2 diagnostic coordinator

`v2_experiment::diagnostic` retains a bounded, unqualified diagnostic event at
each clock in one complete probe manifest. It owns two independent six-branch
families. The ordinary `V2Family` supplies actual accepted states, while the
`ProbeFamily` evolves separately from rest and supplies reconstructed values,
derivatives, accepted-node provenance and off-stage residual inputs.
Every probe publication is also measured by reconstructed-value physical,
pressure and analytical-reference consumers before the owner can advance. All
three use `ProbeFields::value`, never its physical-time derivative as velocity.
The pressure consumer constructs pressure and its gradient on the common doubled
finest retained grid with the original force at that probe clock. The reference
consumer reports velocity, gradient, Hessian and vorticity errors separately for
all six fields on the configured lattice. Those global sampled errors do not
replace accepted regional tracking or establish region coverage.

Admission requires every ordinary accepted clock to appear in the probe
manifest. Every other manifest clock must occur exactly once in the residual
subset, and `ResidualFamilyPlan` independently proves that each such clock has
valid three-node off-stage geometry for all six branches. A manifest such as
`[0, 7, 63, 64, 95, 127, 128]` therefore has accepted paths at `0`, `64` and
`128`, and residual paths at `7`, `63`, `95` and `127`. Stage clocks which are
neither ordinary accepted clocks nor valid residual clocks are refused before
allocation.

At an accepted clock, the driver advances the ordinary family first and the
probe family to that same requested clock. It then retains full-band family
comparisons, physical velocity/gradient/Hessian/vorticity comparisons,
pressure/pressure-gradient comparisons, global and regional binary64
analytical tracking, and an exact retained-node provenance/bitwise finding.
At a residual clock, it advances only the probe family and immediately measures
the physical velocity/ordered-gradient/ordered-Hessian/vorticity differences
and fresh-force doubled-band residual while the reconstructed fields remain
published. The event records the accepted-state path as `NotScheduled` rather than
representing missing data with zeros.

The joint preflight counts the ordinary family once, the complete reconstructed
probe owner once, each consumer's incremental storage, the exact retained event
capacity and two transient event copies at the driver call boundary. Copies
retained by a caller are outside the owner storage contract. Separate finite
ledgers expose probe, accepted physical, reconstructed physical, reconstructed
pressure, reconstructed analytical-reference, accepted pressure, accepted
analytical-reference, regional, residual and binding work. A failed event attempt terminates the driver and
publishes no partial event. Numerical owners, published events and consumer
charges remain inspectable read-only.

Every event has status `UnqualifiedDiagnostic`. The coordinator applies no
threshold and makes no PDE acceptance, accuracy, completeness or convergence
claim. It explicitly lists the absent force-resolution, force-precision,
arithmetic, reference-precision, pressure-reference, pressure-gauge, transfer,
sampling-resolution, quadrature-resolution and region-volume-coverage
channels. Analytical velocity/gradient/Hessian/vorticity tracking is available,
but its reference is evaluated in binary64 on the configured sample grid. The
offline high-precision reference study is not attached. The five named
classifier outputs are exclusive for each sampled point. Their counts do not
prove nominal-region coverage or volume enclosures, and `NoSamples` remains
distinct from a zero error.

Run the exact fixed profile with:

```text
cargo run -p nsbu-benchmarks --example v2_diagnostic_coordinator --release
```

To inspect admission bytes, work ledgers and missing channels without allocating
the driver or evolving either family, append `-- --dry-run`.

The installed public command uses the same owned fixed-array profile:

```text
nsbu diagnose-v2 --dry-run
nsbu diagnose-v2
```

The CLI prints concise diagnostic summaries. Complete raw event reports remain
available through the library API. The versioned full JSON writer in
[`V2_DIAGNOSTIC_EXPORT.md`](V2_DIAGNOSTIC_EXPORT.md) retains those detailed
pair, branch, region, provenance and residual fields. Its stable default schema
version 1 omits the newer reconstructed physical field; callers must explicitly
select version 2 to serialize it.
