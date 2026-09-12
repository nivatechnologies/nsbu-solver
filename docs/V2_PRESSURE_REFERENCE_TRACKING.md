# Exact-v2 startup pressure-reference tracking

`v2_experiment::pressure_reference` is a bounded diagnostic for the fixed startup
accepted clocks 0, 64 and 128 at exponent -20. It constructs numerical pressure
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

The initial allowlist contains only the three JSON artifacts in
`crates/nsbu-benchmarks/data/v2-pressure-gauge`. `ImportedGauge::load` hashes the
actual caller-provided bytes and accepts only those frozen SHA-256 values. The raw
JSON remains authoritative. A generated typed projection exposes all ten raw
means and their axial/radial panel counts and 80/120-digit settings, binary64
roundings, and every separate precision, joint-grid, axial-only and radial-only
change. A test parses the authoritative JSON independently and checks the typed
projection field by field.

Each artifact used base panel count 8, hence joint 8/16/32 and crossed 16x32/32x16
profiles at both precisions. One profile preflight admits 5,162 pressure
evaluations, at most 446,464 scalar-root iterations, and 41,418,752 bytes peak
conservative Python storage. On the recorded machine, dry runs took 0.11-0.13 s;
complete clock 0/64/128 studies took 0.97/1.20/1.18 s and reported 27,648 KiB RSS.
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

This allowlist cannot provide gauges at a later concentrating-window clock such
as 4096. It does not close the review profile's general pressure-reference or
pressure-gauge gaps, define tolerances, establish continuum quadrature accuracy,
or qualify a PDE window. A later profile must generate, preserve, and admit its
own independently refined exact-clock artifact before pressure observations can
be reviewed.
