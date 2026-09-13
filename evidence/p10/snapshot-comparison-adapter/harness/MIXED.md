# Mixed force/space sensitivity diagnostic

The closed `--mixed-force-space` mode accepts exactly three manifests followed by an exact byte
cap:

```text
p10-snapshot-comparison-adapter COARSE.json BASELINE.json FORCE.json CAP_BYTES --mixed-force-space
```

The only admitted ordered triple is N256/M384 Cox-Matthews, N384/M384 Cox-Matthews, and
N384/M512 Cox-Matthews. Case, domain, viscosity, quantum, physical clock, endpoint, schedule, and
tolerances must otherwise match exactly. Each manifest retains its own exact state and coefficient
hashes, plan hash, identity, profile, source, backend, execution binding, and admission guard. All
three file lengths are checked before any state allocation, and all normal state framing, identity,
clock, finite-spectrum, Hermitian, Nyquist, trailer, and whole-file hash checks remain active.

On the full N384 retained band the diagnostic forms

```text
A = U384M384 - lift(U256M384)
B = U384M512 - U384M384
C = U384M512 - lift(U256M384) = A + B.
```

It reports volume-average L2, H1, and vorticity L2 norms for A, B, and C, each split into the N256
common band and the newly resolved N384 shell. For the same splits it reports the weighted signed
cross term `2 Re <A,B>` and the cosine similarity `Re <A,B> / (||A|| ||B||)` when both denominators are
nonzero; an undefined cosine similarity is serialized as `null`. Half-plane Parseval weights restore omitted
conjugate modes and strict Nyquist planes remain excluded.

This is an explicit sensitivity diagnostic with `acceptance: not_assessed`, no assigned budget,
and no spatial-acceptance, force-acceptance, pointwise, time-supremum, finest-grid, or continuum
claim. It performs no FFT, state injection, resume, or trajectory evolution.
