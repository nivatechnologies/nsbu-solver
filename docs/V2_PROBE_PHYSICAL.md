# Sampled physical reconstructed-probe differences

`v2_experiment::probes::physical` measures four physical quantities from the
six actual coefficient fields published by a `ProbeFamily`. It consumes only
`ProbeFields::value`; the reconstructed physical-time derivative is a different
field and is never treated as velocity. The five fixed comparisons are N0/N1,
N1/N2, H0/H1, H1/H2, and CM/HO.

Velocity retains three components, the gradient all nine ordered entries, the
Hessian all 27 ordered entries, and vorticity the complete three-component
curl. Every comparison reports sampled RMS, pointwise Euclidean or Frobenius
error peak, relative-error peak, finer-field reference peak, the exact relative
floor, and lattice witnesses for all three peaks. RMS divides by physical sample
points rather than tensor components.

`ProbePhysicalPlan::new` binds the full probe-family identity and manifest, all
six source domains, one common sample layout, four binary64 floor words, a
maximum whole-attempt count, and the simultaneous probe-owner and consumer
storage cap. Its checked work allowance covers all twenty comparisons and the
publication binding checks. Each attempt reserves 128 conservative whole
identity, clock, origin and domain comparisons; this is not a bytewise or
scalar-word operation count. The consumer reservation includes its shared
transform/reduction workspace, retained report, and 64 bytes for each of 32
allocator-metadata allowance slots. This allowance is not a process-RSS bound.

`ProbePhysicalWorkspace::measure` requires the exact next published probe clock,
the producer's current complete sample, matching identity, origins, domains and
field clocks. It charges a whole attempt before validation. A failed request
terminates the consumer, publishes no partial result, and retains the previous
complete report and charged ledger. The borrowed producer fields and all six
independently integrated states remain unchanged.

The initial bounded profile measures clocks 0, 7 and 128; tick 7 is a genuine
off-stage reconstruction. Tests compare all twenty physical findings and every
reported peak witness against independent signed full-complex Fourier sums at
both non-rest clocks. They also cover exact-rest floor/tie behavior, missing,
foreign, stale and failed publications, attempt and storage refusal, state
integrity, bounded construction, and zero steady measurement allocation.

```sh
cargo test -p nsbu-benchmarks --test v2_probe_physical --locked
cargo test -p nsbu-benchmarks --test v2_probe_physical_allocation --locked
```

Reports carry exact probe clocks and accepted-node origins, domains, sample
layout, floors, probe identity, case hash, and permanent `DiagnosticOnly`
status. They do not estimate reconstruction error, pressure, analytical
reference error, force-grid precision, continuum error, or window qualification.
No policy threshold or PDE success claim is attached.
