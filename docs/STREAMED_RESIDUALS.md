# Streamed independent off-stage residuals

`ResidualFamily` collects conservative PDE defects from all six independently
evolved smooth trajectories while their accepted histories still cover each
required probe. It binds a nonempty, strictly ordered subset of the complete
[streamed probe manifest](RECONSTRUCTED_PROBES.md). Sampling an early defect is
not deferred until later accepted states have overwritten its history.

Each branch reconstructs velocity and its physical-time derivative again in
separate diagnostic scratch. A fresh unprojected force is evaluated at the probe
clock. The existing conservative-product path independently forms:

```text
r = v_t + P div(v tensor v) - nu Delta v - P f
```

The full doubled velocity band is retained. No integration-stage RHS, analytical
reference, stage pressure or interpolated force is substituted. The consumer
borrows the owner immutably and never resets an integrated state or modifies the
owner's already computed interpolants.

## Execute the public workflow

```sh
cargo run --release -p nsbu-benchmarks --example smooth_streamed_residuals -- --dry-run
cargo run --release -p nsbu-benchmarks --example smooth_streamed_residuals
```

The CyclicSine profile uses N=4/8/12, macro ticks H=64/32/16, quantum 2^-16,
endpoint 128, and CM/HO. The complete owner manifest is 0/7/31/63/95/127/128;
the residual subset is 7/31/63/95/127. Unknown or extra command arguments fail.

Every residual probe produces six complete doubled-band defect norms and five
complete residual-field comparisons: N0/N1, N1/N2, H0/H1, H1/H2 and CM/HO.
Reports retain L2, H1, vorticity and divergence norms, the separate common/newly
resolved bands, and the unremoved mean residual difference. The comparison uses
actual residual coefficients; it does not subtract two reported norms. No
alignment, rotation or recentering is applied.

The report also retains the original velocity reconstruction record, every
actual accepted-node triple and current state clock, and three strictly nested
temporal reconstruction geometries. At probe 7 the temporal histories are
0/64/128, 0/32/64 and 0/16/32. All refer to the same physical time despite their
different accepted lookahead endpoints.

## Exact non-stage admission

Residual times must already occur in the full owner manifest. They cannot be
rest, endpoints, interpolation nodes or any full/two-half CM/HO stage clock
within their histories. Stage clocks occur on each macro interval's quarter
lattice; the consumer checks all six histories using exact integer ticks.
Every declared time must also support two strict nested temporal refinements.

If the smallest macro interval is four ticks, every representable tick is a
stage time. The residual subset is refused. Choose a finer exact clock before
the run; the consumer never silently rounds a physical time or migrates a clock.
A missing, duplicate, reversed, nodal or out-of-manifest residual time fails
admission before owner allocation. Sampling geometry alone does not establish
reconstruction accuracy or a continuous-time bound.

## Resources and failure behavior

The public profile jointly reserves 32,411,656 numerical bytes under a 128 MiB
cap. That includes 22,193,768 bytes for all owners/interpolants and 10,217,888 bytes
for six independent residual workspaces and conservative report/header allowance.
Other physical/reference consumers, caller manifests, retained output artifacts
and allocator overhead need separate storage budgets.

Five complete attempted reports reserve 30 child residual calls, 270 scalar
transforms, 488,640 prescribed-force work units and 13,644,800 weighted child
coefficient visits. Five complete field-comparison sets add 1,021,440 comparison
visits; exact manifest binding admits 90 clock comparisons. These distinct units
are not wall-clock predictions or primitive FFT FLOPs. All products/sums and
both complete storage/work limits are checked before allocation.

An aggregate request consumes its complete conservative work allowance before
binding or evaluation. `child_work()` separately reports the child ledgers, so
refusals before numerical work do not masquerade as executed residual calls.
Invalid current owners/probes can be corrected only if an attempt remains.
A numerical child or complete-comparison failure permanently terminates the
consumer, preserves earlier legal state commits and spent work, and publishes
no partial aggregate. It never silently skips a missing earlier residual.

Each completed child must report the same accepted-node origin as the owner
published for that probe. Even a new legal history that still covers the physical
time cannot be relabeled as the previous origin. This check guards the binding
between residual values and their actual reconstruction history.

## Verification and limitations

Tests stream early and late probes, retain exact nested geometry, check nonzero
actual defects, preserve every state/interpolant digest, and demonstrate that
full residual differences differ from scalar norm differences. They reject
invalid subset geometry, resource/work overflow, changed settings, missing/skipped
probes and rejected owners. Private failure tests exercise an exhausted later
child and a newly advanced but mismatched accepted history after earlier children
have already computed valid results.

Allocator instrumentation checks no heap activity during admission, complete
construction within the joint reservation, and no allocations/deallocations/
reallocations during repeated valid or refused observations. Existing independent
conservative-product, projection, residual and Hermite fixtures verify the reused
numerical kernels separately.

The smooth measurements are sampled diagnostics. Some spatial differences are
near binary64 arithmetic noise; no zero floor or convergence certificate is
inferred from them. Complete force, reference, current-grid arithmetic, regional/
location, balance-quadrature and benchmark/artifact integration remain. P08/P09
are incomplete, and no concentrating PDE window is accepted.
