# Exact-v2 startup pressure-reference tracking

`v2_experiment::pressure_reference` is a bounded diagnostic for the fixed startup
accepted clocks 0, 64 and 128, or the reviewed first-window clocks 0, 2048 and
4096, at exponent -20. It constructs numerical pressure
from each of the six actual accepted family states with the existing original-force
Poisson kernel, samples scalar pressure and its full three-component gradient on
the admitted 24-cubed lattice, and compares those samples with the analytical
exact-v2 pressure fields.

The scalar reference subtracts one independently evaluated global unit-cube mean
at every sample point. It never fits a mean to one branch, sample lattice, or
region. The analytical pressure gradient is unchanged by this constant. The
consumer caches analytical values once per clock, completes all six branches,
and only then returns a report. A numerical/reference failure terminates later
publication; attempts and worst-case work are charged before computation.

## Imported empirical gauges

The immutable allowlist contains five JSON artifacts in
`crates/nsbu-benchmarks/data/v2-pressure-gauge`: the original startup clocks
0/64/128 and the first-window clocks 2048/4096. The original three raw files and
their generated entries remain byte-for-byte unchanged. `ImportedGauge::load` hashes the
actual caller-provided bytes and accepts only those frozen SHA-256 values. The raw
JSON remains authoritative. A generated typed projection exposes all ten raw
means and their axial/radial panel counts and 80/120-digit settings, binary64
roundings, and every separate precision, joint-grid, axial-only and radial-only
change. A test parses the authoritative JSON independently and checks the typed
projection field by field.

Each startup artifact used base panel count 8, hence joint 8/16/32 and crossed
16x32/32x16 profiles at both precisions. Each first-window artifact used base
panel count 32, hence joint 32/64/128 and crossed 64x128/128x64 profiles. The
larger profile admits 77,450 pressure evaluations, at most 1,724,416 scalar-root
iterations, and 41,418,752 bytes peak conservative Python storage. The new clock
2048 study completed in 12.63 seconds and reported 27,648 KiB RSS; clock 4096 is
the previously preserved endpoint artifact with SHA-256
`dff3b5075ca84c7c6698875e0613be4149bea7eafc000624231fda04cb25f84d`.
These are empirical quadrature refinements, not interval enclosures or selected
pressure-error budgets.

## Resource and scientific boundary

The Rust plan explicitly reserves every owned complex field, real scratch vector,
`ReferenceEvaluation` cache entry, FFT/derivative/conservative workspace,
original-force provider, vector header, report, and allocator allowance. Imported
artifact hashing is one-time caller work retained separately from the per-attempt
numerical ledger. That ledger includes provider work, all six conservative
pressure constructions, all derivative transforms and a conservative visit
envelope inherited from the larger ten-construction pressure-pair consumer; as
elsewhere, weighted visits exclude internal FFT butterfly accounting. The
startup allocation test measures construction against that
consumer reservation and observes zero steady allocations for a complete rest
report.

The allowlist supplies only its five exact clocks. Admission requires a family
manifest whose three clocks exactly match one ordered gauge triple; it does not
interpolate a gauge or accept a nearby time. The first-window fixture also binds
N=12/16/24, steps 64/32/16, force M=24 with 12 workers, endpoint 4096, and a
48-cubed pressure sample/construction layout through the admitted family and
pressure plans. It does not close general pressure-reference or pressure-gauge
gaps, define tolerances, establish continuum quadrature accuracy, or qualify a
PDE window. A later profile must generate, preserve, and admit its own
independently refined exact-clock artifact before pressure observations can be
reviewed.

The bounded Rust checks construct and measure the reviewed profile at rest. A
separate coarse diagnostic-only N=4/8/12, M=12 family evolves directly from rest
and measures the nonzero clock-2048 gauge while verifying that observation does
not change any spectral-state hash. It does not evolve that fixture to clock
4096; the clock-4096 result in this increment is import, metadata, and exact-plan
admission evidence backed by the independently preserved Python artifact.
