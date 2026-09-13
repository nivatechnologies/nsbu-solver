# Immutable N384 regional snapshot diagnostic

This bounded adapter reads either of the two explicitly reviewed N384 clock-512 snapshots: the
historical M384 integration-force baseline or the independently evolved M512 integration-force
history. It samples velocity, ordered gradient, and all 27 ordered Hessian entries on the same
unshifted 768³ lattice. It retains a
30-word analytical cache per point: velocity `[3]`, gradient `[9]`, and the six symmetric Hessian
entries for each velocity component `[18]`. Every cache insertion compares the expanded symmetric
mapping bitwise against all 27 independently evaluated ordered Hessian entries.

The result contains global errors and sampled errors/counts for core, annulus, interior outside the
nominal regions, cutoff collar, and exterior. Collar physical-volume coverage and peak
qualification are explicitly absent. Core and annulus coverage use the existing 256/512/1024-panel
refinements. The result always carries `acceptance.status=not_assessed` and `accepted_windows=0`.

The bound preflight does not read the staged state payload. The historical M384 binding remains:

```text
cargo run --release --manifest-path evidence/p10/n384-regional-snapshot-diagnostic/harness/Cargo.toml -- \
  preflight evidence/p10/external-reference-bridge/inputs/clock0512/snapshot.json 128771370072
```

The matched M512 binding uses the existing reviewed comparison manifest without changing it:

```text
cargo run --release --manifest-path evidence/p10/n384-regional-snapshot-diagnostic/harness/Cargo.toml -- \
  preflight evidence/p10/external-reference-bridge/inputs/clock0512-m512/snapshot.json 128771370072
```

The M512 state was integrated independently from rest for eight accepted h64 steps. Its exact
profile is `n384-m512-h64to2048-h128to4096-cadv33-w3-f13c29c`, source is
`326eeb5cbd5ebe39a7d5f7be77f9acfab8d0db72`, frozen plan SHA-256 is
`2be3880204aab5da1819e11ed6abb377e43b814f8ef17869d76463f72a33cf84`, and state-file SHA-256 is
`7a1d8d21e17c85c7f37ea474f5f5e694a91889ebabcec12424427308d020def9`. Admission refuses any
cross-pairing of the M384 and M512 profile, source, plan, coefficient, or file identities.

The representative pilot evaluates and classifies every point on the complete x=0 plane, which
contains samples from all five spatial classes:

```text
cargo run --release --manifest-path evidence/p10/n384-regional-snapshot-diagnostic/harness/Cargo.toml -- pilot
```

Do not launch the complete diagnostic while residual localization or another campaign job is
active. A later root-reviewed launch must supply the staged snapshot, expose at least 32 CPUs, have
`MemAvailable >= 145951239256`, apply an external 137438953472-byte address-space limit and a
2400-second timeout, and set all reviewed execution gates:

```text
REGIONAL_MANIFEST=evidence/p10/n384-regional-snapshot-diagnostic/harness/Cargo.toml
cargo build --release --manifest-path "$REGIONAL_MANIFEST"
REGIONAL_BINARY=evidence/p10/n384-regional-snapshot-diagnostic/harness/target/release/p10-n384-regional-snapshot-diagnostic
REGIONAL_BINARY_SHA256=$(sha256sum "$REGIONAL_BINARY" | cut -d' ' -f1)
REGIONAL_SNAPSHOT=evidence/p10/external-reference-bridge/inputs/clock0512-m512/snapshot.json
P10_ROOT_FULL_RUN_REVIEW=approved \
P10_RESIDUAL_LOCALIZATION_IDLE=1 \
P10_CPU_WORKERS=32 \
P10_DIAGNOSTIC_BINARY_SHA256="$REGIONAL_BINARY_SHA256" \
timeout --signal=TERM --kill-after=60s 2400s \
prlimit --as=137438953472 -- "$REGIONAL_BINARY" execute \
  "$REGIONAL_SNAPSHOT" \
  128771370072 OUTPUT.json --root-reviewed
```

The execute path validates the fixed manifest/profile/source/plan/clock hashes before decoding,
uses immutable coefficient borrows after decode, and rehashes the coefficient bytes after all
measurements. Output is written to a bounded candidate, synced, and atomically renamed only after
the diagnostic is complete.

The root-reviewed M512 run completed in 1844.53 seconds with result
`results/clock0512-m512.json`. The result remains diagnostic-only. The direct comparison with the
historical M384 result is recorded in `m384-vs-m512-clock0512.tsv`; M512 reduced every headline
error, while ordered-Hessian collar and exterior errors remained the same order.

The separately identified analytical projection path accepts no snapshot or state argument. It
samples analytical velocity on the unshifted 768³ lattice, applies the reviewed normalized forward
FFT and strict 768-to-384 crop, drops producer scratch, then passes only the projected coefficients
to the unchanged regional measurement path. Its origin is `sampled_analytic_projection`; it makes
zero trajectory-from-rest, state-import, resume, or acceptance claims.

```text
cargo run --release --manifest-path evidence/p10/n384-regional-snapshot-diagnostic/harness/Cargo.toml -- \
  projection-preflight
```

A later root-reviewed execution uses the same conservative cap and launch environment as the
snapshot diagnostic, but has no input-manifest parameter:

```text
"$REGIONAL_BINARY" projection-execute 128771370072 OUTPUT.json --root-reviewed
```

The analytical producer peak is 19533843144 bytes. Its worker-stack sampling subphase ends before
the FFT projection subphase. The projected coefficients survive into measurement after all other
producer allocations drop. The measurement peak is 128770321496 bytes, exactly the historical
measurement reservation with the 1048576-byte snapshot decoder overhead removed. The external cap
and minimum-memory gate remain conservatively unchanged.

The root-reviewed analytical projection run completed in 1884.70 seconds with result
`results/clock0512-analytic-n384-projection.json`. Its projected coefficient hash was unchanged
through measurement, all reported values were finite, and the five region counts matched the two
actual-state diagnostics exactly. The matched actual-M512 comparison is recorded in
`m512-vs-analytic-n384-projection-clock0512.tsv`. The gradient and ordered-Hessian norms and peaks
are closely comparable, including the collar and exterior regions, which strongly supports the
retained N384 representation as the dominant limitation in these derivative diagnostics. Ratios
are not an exact error decomposition because cross terms and local cancellation remain possible.
This diagnostic makes no continuum, peak-qualification, or acceptance claim.
