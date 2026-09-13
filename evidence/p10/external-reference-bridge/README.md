# P10 read-only external reference bridge

This bounded bridge compares one reviewed `P10AVXSNAP1\0` N384 state with an
independently sampled exact-v2 analytical velocity at the same exact clock. It
does not construct a trajectory family, resume or import state into the solver,
write a snapshot, alter review geometry, assign a reference field, or accept a
PDE window.

The first frozen input is clock 512. Its snapshot manifest preserves the complete
N384 trajectory identity and hashes, and expects the remotely retained state to
be staged at `inputs/clock0512/staged/n384-state.bin`. The required file SHA-256
is `951be3d85acd3179c2e152a11220e5231ead5d5b8a709217da83b988e9abc243`;
the coefficient SHA-256 is
`5f559ad2e80747f102c1bf426211ca2313a89ac63b35cfecf4db723aaf57b44a`.
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

The classification is a sampled binary64 reference diagnostic. It is not a
continuum reference, precision refinement, enclosure, current-grid arithmetic
bound, convergence result or accepted window. The exact preflight reserves
20,900,924,104 bytes for the decoded state, all three M768 physical components,
one catalog-backed scalar FFT workspace and output, three retained reference
components, allocator allowance and bounded serialization. It charges
452,984,832 point evaluations and at most 57,982,058,496 scalar root iterations.
No M768 evaluation was run while preparing this increment.

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
The bridge manifest also hash-checks the snapshot manifest, analytical evaluator
source and frozen trajectory plan. A later physical diagnostic can reuse this
same unshifted point traversal to evaluate the existing analytical gradient,
ordered Hessian and vorticity producer, then feed the existing regional reducers.
That extension is documentation only here; this bridge owns no unused hooks or
partial producer scaffolding.
