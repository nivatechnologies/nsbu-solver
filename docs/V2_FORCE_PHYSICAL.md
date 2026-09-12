# Sampled physical force-grid differences

`v2_force_experiment::physical` measures the effect of the prescribed-force
sampling grid on four physical quantities from the three independently evolved
`ForceFamily` states. It uses one declared physical lattice for both M0/M1 and
M1/M2. Velocity has three components, the gradient retains all nine ordered
entries, the Hessian retains all 27 ordered entries including both mixed-entry
orders, and vorticity retains the complete three-component curl.

Each pair reports sampled RMS error, sampled pointwise Euclidean or Frobenius
peak error, peak relative error, finer-state reference peak, the explicit
relative floor, and the first lattice point attaining each peak. RMS divides by
the number of physical sample points, not by the number of components. M1 is the
reference side of M0/M1 and M2 is the reference side of M1/M2. These are direct
physical comparisons; the consumer never converts spectral H1 values into
gradient or Hessian errors.

`ForcePhysicalPlan::new` binds the complete force-family identity and manifest,
the exact fixed numerical and force-grid profile, common sample layout, four
binary64 floor words, maximum whole attempts, and simultaneous family/consumer
storage. Its checked work allowance includes both pairs for all quantities,
complete inverse transforms, sampled reductions, and all extrema scans.
Allocation occurs only in `ForcePhysicalWorkspace::new`; successful and failed
measurement attempts perform no allocation. The storage declaration adds 64
bytes for each of 32 conservative allocator-metadata slots around the shared
workspace allocations. This is an admission allowance, not a process-RSS bound.

`measure` charges a complete attempt before validating the supplied raw family
publication. It then requires the exact next family clock, identity, and three
synchronized caller-owned states. The report is published only after all eight
quantity/pair comparisons succeed. A failed request terminates the consumer and
retains the previous complete report and complete charged ledger. The family
states remain read-only and receive no analytical assignment or reset.

Every report carries the case hash, family identity, exact clock, force-family
settings, sample layout, floor words, and permanent `DiagnosticOnly` status.
No force-precision, sufficiency, monotonic convergence, continuum, regional, or
accepted-window conclusion is produced. Pressure is outside this consumer.

Focused verification uses:

```sh
cargo test -p nsbu-benchmarks --test v2_force_physical -- --test-threads=1
cargo test -p nsbu-benchmarks --test v2_force_physical_allocation -- --test-threads=1
```

The oracle independently performs signed full-complex Fourier sums at every
physical sample and constructs velocity, every ordered gradient and Hessian
entry, and curl. It checks actual nonzero endpoint RMS and peaks for both pairs.
Additional controls cover rest, missing/foreign/stale/failed family
publications, attempt exhaustion, cap and floor refusal, prior-report retention,
state immutability, bounded construction, and zero steady-state allocation.
