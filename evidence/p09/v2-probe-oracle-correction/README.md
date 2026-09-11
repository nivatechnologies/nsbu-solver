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

Result: 2 passed, 0 failed.  The late-clock maxima were:

- reconstructed value signal: `1.70239862618906e-10`
- reconstructed derivative signal: `4.047231179764178e-5`
- value error: `3.900387500920336e-12`
- derivative error: `3.257386210248468e-6`
- wrong-coordinate value gap: `1.7202700637431393e-10`
- omitted-RHS value gap: `3.8118839030161114e-11`
- wrong-coordinate derivative gap: `4.040830983223315e-5`
- omitted-RHS derivative gap: `3.183349185683363e-5`

Targeted Clippy also passed with warnings denied.  The test source SHA-256 is
`b4bfe4dcdc1e2331c70c6256f047ecfd3abc5af56d7f45ce27f47ec22ae32039`.
