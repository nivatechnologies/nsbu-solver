# Exact-v2 late Hermite oracle correction

This regression check replaces the original near-rest clock-7 oracle with an
actual clock-95 probe reconstructed from independently evolved accepted nodes
at clocks 64, 80, and 96.  The test solves the six Hermite weights with a dense
linear system and checks both the value and time derivative.  It never seeds a
state or imports reconstructed fields.

The node right-hand sides are freshly assembled with `V2Force` and
`ConservativeWorkspace`, independently of the observer's stored nodes.  This
is an independent interpolation oracle, not an independent implementation of
the PDE operator.

Command:

```text
cargo test -p nsbu-benchmarks --test v2_probe_family --locked -- --nocapture
```

## Superseded first run

Commit `adb96ef` initially sampled the fresh node RHS force on each branch's
`2N` diagnostic grid.  That did not reproduce the reconstruction observer's
configured `M=24` force input profile.  Its `3.900387500920336e-12` value error
and `3.257386210248468e-6` derivative error therefore mixed force-sampling and
interpolation differences.  Those figures and the loosened bounds in that
commit are withdrawn as gate evidence.

## Corrected run

The corrected oracle uses the observer's configured `force.samples` doubled
to `M=24`, then transfers each freshly evaluated RHS through the branch's
diagnostic workspace.  Result: 2 passed, 0 failed.  The late-clock maxima were:

- reconstructed value signal: `1.6966154415435105e-10`
- reconstructed derivative signal: `4.0955312033235516e-5`
- value error: `2.6253290925755273e-26`
- derivative error: `6.829203137237796e-21`
- wrong-coordinate value gap: `1.7177409977117067e-10`
- omitted-RHS value gap: `3.869715749471606e-11`
- wrong-coordinate derivative gap: `4.090370945821906e-5`
- omitted-RHS derivative gap: `3.231649209242737e-5`

Targeted Clippy also passed with warnings denied.  The test source SHA-256 is
`d243e62c6697588849928ac1f4c4f059adf5697544aec5082da0c3eb8f1b142a`.
