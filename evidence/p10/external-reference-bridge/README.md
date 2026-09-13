# P10 read-only external reference bridge

This bounded bridge compares one reviewed `P10AVXSNAP1\0` N384 state with an
independently sampled exact-v2 analytical velocity at the same exact clock. It
does not construct a trajectory family, resume or import state into the solver,
write a snapshot, alter review geometry, assign a reference field, or accept a
PDE window.

The frozen inputs cover clocks 512 and 4096. Their manifests preserve the complete
N384 trajectory identity and hashes, and expect the remotely retained states under
each input's `staged/n384-state.bin`. Clock 512 binds whole-file SHA-256
`951be3d85acd3179c2e152a11220e5231ead5d5b8a709217da83b988e9abc243` and
coefficient SHA-256 `5f559ad2e80747f102c1bf426211ca2313a89ac63b35cfecf4db723aaf57b44a`.
Clock 4096 binds whole-file SHA-256
`2868bc6e5ccbfb5ce5967aefd3d82cccbaec0a72baaeaad0944f517948187592` and
coefficient SHA-256 `921e2e3b83eea4b259d9794eb0e663ce8f8f913322283312997820e31a1cb72b`.
The decoder verifies length, magic, identity, exact clock words, coefficient
trailer, whole-file hash and strict spectrum before numerical reference work.

The producer calls the existing explicit scalar exact-v2 evaluator at the
unshifted periodic points `(i/768,j/768,k/768)`. It retains raw binary64 velocity
samples, performs three normalized RustFFT forward transforms, and uses the
existing normalization-preserving strict transfer from M768 to N384. It applies
no coordinate shift, fit, mean alignment, filtering or Leray projection. Target
Nyquist coefficients are explicit zeros. Full retained-band L2, inhomogeneous
H1, curl L2 and divergence L2 differences include the zero mode and every strict
N384 mode. Separate actual and sampled-reference norms expose raw divergence.

The analytical source closure is fixed at commit `6bdea3084d737ce6585cb67ab5d48810bd03cd50`
and hashes the scalar evaluator, root solver, clock conversion, error type, module
root, tick clock, and byte-preserved case definition. The executed binary SHA-256
is `a39b4811138a0f5dd39540e4bc70b83f5f48b8bce5de1c5b822b420ace201047`,
built from bridge source commit `47a4d9c7096d1e02c9ce995bbd61514a8dede1d2`.

The classification is a sampled binary64 reference diagnostic. It is not a
continuum reference, precision refinement, enclosure, current-grid arithmetic
bound, convergence result or accepted window. The exact preflight reserves
20,900,924,104 bytes for the decoded state, all three M768 physical components,
one catalog-backed scalar FFT workspace and output, three retained reference
components, allocator allowance and bounded serialization. It charges
452,984,832 point evaluations and at most 57,982,058,496 scalar root iterations.
The completed clock-512 run took 119.99 seconds and 20,383,744 KiB maximum RSS.
The completed clock-4096 run took 117.44 seconds and 20,379,648 KiB maximum RSS.
Both exited zero, passed independent result validation, and were promoted only
after their source, binary, snapshot, and report bindings matched.

| clock | difference L2 | difference H1 | difference curl L2 | difference divergence L2 |
| ---: | ---: | ---: | ---: | ---: |
| 512 | 6.933499382700655e-8 | 6.749991863805842e-5 | 6.749988304326447e-5 | 6.170649568650966e-10 |
| 4096 | 9.10406983035357e-7 | 1.0804989124323632e-3 | 1.080498529134934e-3 | 9.924936495841918e-9 |

The command defaults to preflight. `--execute` is an explicit heavy-work gate:

```sh
cargo run --release --manifest-path evidence/p10/external-reference-bridge/harness/Cargo.toml -- \
  evidence/p10/external-reference-bridge/inputs/clock0512/bridge.json \
  20900924104

cargo run --release --manifest-path evidence/p10/external-reference-bridge/harness/Cargo.toml -- \
  evidence/p10/external-reference-bridge/inputs/clock0512/bridge.json \
  20900924104 --execute
```

Before execution, stage the exact remote state and rerun the preflight command.
The scripts under `run-support` impose the 1,800-second timeout, retain GNU time
and phase evidence, write a temporary candidate, and promote only after the
validator accepts the complete report. The clock-512 lifecycle archive records
an earlier valid candidate whose wrapper exit was not observed; it remained
unpromoted, and the clean closure output was byte-identical.

The bridge manifest also hash-checks the snapshot manifest, complete analytical
source closure and frozen trajectory plan. A later physical diagnostic can reuse this
same unshifted point traversal to evaluate the existing analytical gradient,
ordered Hessian and vorticity producer, then feed the existing regional reducers.
That extension is documentation only here; this bridge owns no unused hooks or
partial producer scaffolding.
