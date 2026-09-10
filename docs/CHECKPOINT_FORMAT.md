# Experimental checkpoint formats

This document describes the formats implemented at the time of writing. They are experimental
implementation formats with consolidated local runtime checks. They are useful for bounded recovery of
the implemented components, but do not establish a qualified restart or a PDE result.

All multibyte integers are little-endian. Integer fields described as `u128` are encoded in 16
bytes even when the in-memory value is `usize`; a reader refuses values that do not fit its local
`usize`. Every floating-point field is the little-endian `u64` result of `f64::to_bits`.
Finite values, including signed zero, retain their bits; physical fields reject nonfinite values. Lengths,
counts, and complete input byte limits are checked before input-sized allocation. A short output
buffer is refused before the writer changes it. Readers reject truncation, trailing bytes,
unknown tags, and unsupported versions.

## Artifact archive: `NSBUAR01`

[`checkpoint/archive.rs`](../crates/nsbu-solver/src/checkpoint/archive.rs) stores a canonical
catalog of six required artifacts and one optional reference artifact. Its bytes are:

```
8 bytes magic | 1 byte entry count | entries in canonical order
entry = 32 byte SHA-256 | u128 content length | exact content bytes
```

The entry count is six or seven. The required order is problem, execution, force, force
coverage, policy, lineage, then optional reference. The catalog reader verifies each nonempty
entry's SHA-256 digest and rejects duplicate, missing, reordered, oversized, or malformed
entries. A hash match establishes integrity of supplied bytes only. It does not establish that
the artifact is truthful, adequate, associated with the physical payload, or sufficient for a
qualified continuation. This format has no separate numeric version field: `AR01` is its schema
version. A schema change requires a new magic and a reader that explicitly supports it.

## Raw run-history archive: `NSBUHR01`

[`checkpoint/history.rs`](../crates/nsbu-solver/src/checkpoint/history.rs) writes a 157-byte
header followed by fixed 176-byte attempt records defined in
[`checkpoint/record.rs`](../crates/nsbu-solver/src/checkpoint/record.rs):

```
8-byte magic
initial clock: i32 exponent, u128 target, u128 elapsed, u128 remaining
method tag
u128 endpoint, u128 step ticks, u128 maximum attempts
four f64-bit tolerance values
u128 record count
record[count]
```

Method tag `1` is Cox--Matthews and `2` is Hochbruck--Ostermann. Each record retains its exact
start clock, outcome/refusal tag and stable failure code, optional indicators, and optional
balance sample. Absent optional values have canonical all-zero payload bytes. On import, the
reader preflights byte, record, replay, and storage limits, reconstructs the controller and
compensated history in original order, and reports replay/clock violations as `InvalidHistory`.
It does not import physical state, provenance, provider state, or a numerical qualification.

## Physical payload: `NSBUPH01`

[`checkpoint/physical.rs`](../crates/nsbu-solver/src/checkpoint/physical.rs) is versioned with
`u16` version `1`. Its 350-byte header contains the complete identity checked against an
independently approved `ResourcePlan`: three `u128` grid dimensions; three f64-bit domain
lengths; f64-bit viscosity; plan epoch; eight `u128` resource classes; total planned bytes;
clock exponent/target/elapsed/remaining; state epoch; accepted-step count; and the `u128`
half-spectrum coefficient count. It is followed by all three Fourier components, in component
and stored-layout order, each coefficient as real then imaginary f64 bits.

The reader requires matching domain, layout-derived count, plan epoch, all resource classes and
total before allocating field storage. It restores exact clock counters and returns
`UnverifiedPhysical`, which exposes read-only state and an ownership transfer only. It checks
finite coefficients, componentwise Hermitian conjugacy with the same absolute `1e-12` tolerance
used for accepted attempt fields, and **exactly zero** coefficients on every Nyquist plane. It
never repairs, projects, rounds, or normalizes an imported spectrum. A malformed physical
payload should be discarded and recovered from another complete source; it must not be patched
in place.

## Smooth-run owner archive: `NSBUSR01` and the origin boundary

[`smooth_run/archive.rs`](../crates/nsbu-benchmarks/src/smooth_run/archive.rs) stores the balance-only
owner container. It has `u16` version `1`, a 264-byte header, the embedded physical
and history archives, one 48-byte integration-work record per history record, and a 32-byte
SHA-256 trailer over every preceding byte. Its header records an origin tag, initial clock,
configuration, observer sample limit, advective-limit f64 bits, embedded physical/history byte
lengths, work count, and three observer-consumption counters.

The reader verifies the trailer and all nested sizes before allocation, requires the expected
plan, replays history, checks physical/controller clock and committed-count agreement, validates
the fixed `CyclicSine` provider work accounting, and restores observer state. This owner format
currently supports **only `CyclicSine`**. Reconstruction history is inactive and is not encoded.

The encoded origin tag records whether the writer's in-memory run began from rest or was already
external. It crosses an intentional origin boundary on import: every decoded owner becomes
`ExternalUnverified`, regardless of that tag. It may continue only as an externally unverified
diagnostic run with fresh private scratch. This prevents a valid byte stream from silently
becoming a provenance or acceptance claim.

## Errors, recovery, and qualification

`ResourceLimit` means a declared input, record, byte, or storage bound was exceeded; raise an
explicit caller cap only after deciding that the larger resource use is acceptable. `HashMismatch`
means discard the artifact or smooth-owner input and obtain an intact copy. `InvalidCatalog` and
`InvalidEncoding` mean the bytes are malformed, noncanonical, unsupported, or incompatible with
the expected plan; do not attempt byte-level repair. `InvalidHistory` means decoding succeeded
far enough to replay but the reconstructed controller/history invariants failed; recover a
coherent earlier set rather than combining components from different runs.

A same-profile restart only says that the imported bytes match the expected execution profile and
pass these local structural checks. It does not authenticate external provenance, prove artifact
semantics, restore inactive reconstruction evidence, establish reference agreement, or qualify a
PDE window. The separate experimental `NSBULN01` lineage event archive preserves operation
order and replays declarations through the checked registry; it does not contain
physical states or the contents of external transfer-error reports.

## Accepted reconstruction snapshots

`ReconstructedRun` uses a separate trusted in-memory `OwnedSnapshot` containing
its three accepted endpoint values and independently evaluated derivatives. Each
node retains its exact clock, state epoch and committed-step count. The snapshot
also retains consumed provider work and modal visits. Capture refuses a pending
proposal; restoration checks the final node against the physical state before
allocating fresh diagnostic scratch. Snapshot reservation includes all copied
fields before any allocation.

These snapshots are not standalone `NSBUSR01` payloads. The balance-only writer
accepts only the concrete `SmoothRun`; the separate reconstruction formats below
preserve active history. Neither authenticates a concentrating comparison lineage
or establishes PDE convergence.

## Accepted-node component: `NSBURN01`

The [node codec](../crates/nsbu-benchmarks/src/smooth_observer/reconstruction/archive.rs)
has a 107-byte header: eight magic bytes, `u16` version 1, `u128` sample capacity,
one-byte active-node count, three `u128` provider/transform counters, `u128` modal
visits and `u128` half-spectrum length. One to three nodes follow. Each node has
an 84-byte exact-clock/epoch/committed-count header, then three velocity and three
independent derivative arrays of complex binary64 coefficients. A node therefore
uses `84 + 96 * half_len` bytes. There is no component checksum.

Readers require a separately approved plan and physical state. They check the
complete length and storage cap, finite strict spectra, consecutive epochs and
step counts, common clock family, actual zero rest values when that node is
present, equal spacing and exact final-state coefficient bits. Decoding produces
`UnverifiedReconstruction`; it cannot create a trusted owned from-rest snapshot.
The enclosing owner must bind these bytes to its complete execution history.

## Reconstruction owner: `NSBURC01`

The [owner codec](../crates/nsbu-benchmarks/src/smooth_run/reconstructed_archive.rs)
uses a 42-byte header: eight magic bytes, `u16` version 1, and two `u128` payload
lengths. The common physical/history/work frame and `NSBURN01` component follow;
a final 32-byte SHA-256 covers all preceding bytes. The common frame retains the
version-one field layout and its inner checksum, but uses the reconstruction
resource profile and includes the initial-rest observation charge. It cannot be
imported through the public balance-only reader.

The owner checks both frames against one plan, physical state, configuration and
work ledger. Every retained node time must equal its committed-step count times
the configured step. This additional check rejects equally spaced, rehashed node
timestamps that do not match actual recorded step times. Aggregate preflight
includes decoded node storage and fresh execution scratch before allocation.

`ImportedReconstructedRun::continue_unverified` preserves physical bits, accepted
nodes, compensated balances, controller state and spent work. Import, continuation,
snapshot and subsequent export retain `ExternalUnverified`. External problem,
execution, force, policy and lineage artifacts still need semantic binding before
any qualification decision. The current CLI file commands use only `NSBUSR01`;
`NSBURC01` is available through the Rust library.

## Reproduce an imported reconstruction run from rest

The reconstruction reader validates byte integrity, bounded shapes, clock/epoch
coherence and raw-history replay. It cannot infer that supplied field coefficients
were generated by the prescribed PDE. For the fixed smooth `CyclicSine` profile,
`smooth_run::replay::ReplayPlan` provides a separate numerical check:

1. Borrow the imported `ReconstructedRun` without changing it.
2. Admit the exact recorded attempt count and simultaneous storage for the original
   owner, a fresh owner and two complete canonical archives.
3. Construct a new rest state under the decoded immutable configuration and the
   current executable's smooth force/operator implementation.
4. Reproduce every recorded attempt, including terminal rejections and refusals.
5. Compare every canonical archive byte: physical fields, accepted reconstruction
   values and independent derivatives, exact clocks, controller/balance history,
   work ledger, sample allowance and resource profile.

A mismatch returns `DifferentReplay`; the original remains unchanged. The two
encodings use the original owner's current diagnostic-origin tag solely to make
that metadata identical for comparison. This does not alter either owner's
origin. Successful `VerifiedReplay` contains the newly integrated from-rest run
and a digest/count report. `into_run` returns that new run with its original
remaining allowances; it never resets an imported trajectory or assigns an
analytical reference. The borrowed external owner remains `ExternalUnverified`.

This check is deliberately as expensive as reproducing the recorded evolution.
It has explicit finite work/storage bounds and refuses a partial attempt budget.
Byte equality is scoped to the executing implementation. Different hardware,
compiler or arithmetic choices can produce a mismatch; there is no tolerance
that silently accepts altered checkpoint bytes. Matching replay does not qualify
PDE convergence or authenticate an external compiler, policy, reference or force
coverage artifact. The complete concentrating workflow still needs those bindings.

The negative control changes an earlier accepted derivative, recomputes the outer
checksum and successfully imports the structurally coherent file. Actual replay
then rejects it. CM/HO tests also cover every ring phase, next-attempt equivalence,
terminal failures, origin preservation and complete simultaneous storage admission.
