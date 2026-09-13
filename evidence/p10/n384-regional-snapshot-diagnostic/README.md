# Immutable N384 regional snapshot diagnostic

This bounded adapter reads only the reviewed N384/M384 clock-512 snapshot and samples velocity,
ordered gradient, and all 27 ordered Hessian entries on the unshifted 768³ lattice. It retains a
30-word analytical cache per point: velocity `[3]`, gradient `[9]`, and the six symmetric Hessian
entries for each velocity component `[18]`. Every cache insertion compares the expanded symmetric
mapping bitwise against all 27 independently evaluated ordered Hessian entries.

The result contains global errors and sampled errors/counts for core, annulus, interior outside the
nominal regions, cutoff collar, and exterior. Collar physical-volume coverage and peak
qualification are explicitly absent. Core and annulus coverage use the existing 256/512/1024-panel
refinements. The result always carries `acceptance.status=not_assessed` and `accepted_windows=0`.

The bound preflight does not read the missing staged state payload:

```text
cargo run --release --manifest-path evidence/p10/n384-regional-snapshot-diagnostic/harness/Cargo.toml -- \
  preflight evidence/p10/external-reference-bridge/inputs/clock0512/snapshot.json 128771370072
```

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
P10_ROOT_FULL_RUN_REVIEW=approved \
P10_RESIDUAL_LOCALIZATION_IDLE=1 \
P10_CPU_WORKERS=32 \
P10_DIAGNOSTIC_BINARY_SHA256="$REGIONAL_BINARY_SHA256" \
timeout --signal=TERM --kill-after=60s 2400s \
prlimit --as=137438953472 -- "$REGIONAL_BINARY" execute \
  evidence/p10/external-reference-bridge/inputs/clock0512/snapshot.json \
  128771370072 OUTPUT.json --root-reviewed
```

The execute path validates the fixed manifest/profile/source/plan/clock hashes before decoding,
uses immutable coefficient borrows after decode, and rehashes the coefficient bytes after all
measurements. Output is written to a bounded candidate, synced, and atomically renamed only after
the diagnostic is complete.
