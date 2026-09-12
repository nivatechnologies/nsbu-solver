# N384 every-step snapshot I/O feasibility

This isolated harness measures a full three-component N384 half-spectrum snapshot
without changing the active N192/N256 runs or any production API. The input is a
prepared all-zero spectral field, not an integrated trajectory state or numerical
evidence. It uses the endpoint snapshot shape and a harness-owned schema, streams
an 8 MiB encoded-byte buffer through SHA-256 and a buffered file, calls `sync_all`,
publishes the staged bundle by one directory rename, syncs the parent directory,
then reads and validates the complete hash. The bulk bundle is deleted only after
validation.

## Size and admission

The N384 half layout has 28,459,008 complex values per component. Three components
contain exactly 1,366,032,384 coefficient bytes; the measured snapshot file is
1,366,032,589 bytes. With 8 KiB for the attempt and record plus a conservative
64 KiB filesystem allowance, the per-step bound is 1,366,106,317 bytes.

| Committed steps | Exact bound | Decimal approximation | Binary size | Required profile |
|---:|---:|---:|---:|---:|
| 64 | 87,430,804,288 B | 87.43 GB (the planning estimate was about 89 GB) | 81.43 GiB | 128 GiB |
| 128 | 174,861,608,576 B | 174.86 GB (about 177 GB) | 162.85 GiB | 256 GiB |

The 128 GiB cap is 137,438,953,472 bytes: it admits 64 bundles and refuses 128.
The 256 GiB cap is 274,877,906,944 bytes and admits 128. Before measurement,
`/mnt/niva-array` had 7,300,024,086,528 bytes available, so either frozen profile
fits the filesystem with substantial headroom. Admission must still reserve the
whole selected profile before state allocation. Node zero needs only its exact
REST record; scheduled positive nodes reuse that step's bundle and do not create
a second state snapshot.

## Measurement

The source-matched release run occurred while the N192 and N256 endpoint jobs were
active. It wrote and file-synced the snapshot plus metadata in 3.284829158 seconds,
renamed and directory-synced it in 0.001257468 seconds, and validated the SHA-256
in 1.436919749 seconds. Complete process wall time was 4.75 seconds, user time
2.40 seconds, system time 0.86 seconds, and maximum RSS 16,380 KiB. `/proc/self/io`
reported 1,366,044,672 bytes written. It reported zero storage reads because the
validation was satisfied from page cache, so the validation timing is not a cold
read benchmark. The coefficient digest was
`65db2aea053eb94f3d5b9c5c2dd8bd4be8dae247064a40cc13316441afa54460`.

This measures the storage, hashing, sync, and publication path for already encoded
zero coefficients. A real-state implementation must encode finite `f64` words in
fixed little-endian order into the same bounded buffer; that conversion cost and
memory-bandwidth interaction are not measured here. No whole-state copy is needed.

The available N384 AVX projection is 6,712 seconds per step before the later
composite improvements. Snapshot write and durable publication are 0.049% of that
projection; the full measured process including cached readback is 0.071%. Even
against a future three-minute step, the write path is about 1.83%. The I/O cost is
small enough to make every-step snapshots useful for offline diagnostics and
failure-localized review. The artifact reservation, rather than runtime, is the
material cost.

## Transaction design

The N384 harness should own one immutable, nonresumable artifact profile containing
the retained layout, backend/provider identities, step count, observer schedule,
snapshot schema, byte cap, endian encoding, and hash algorithm. An artifact from
this profile must not enter the existing Run/checkpoint resume path.

For each accepted proposal, the owner uses this order:

1. `prepare_commit` exposes borrowed proposal coefficients and clock without
   mutating committed state.
2. It creates `step-N.partial`. Every step stages `state.bin` and `attempt.json`.
   A scheduled step runs the observer against the proposal and adds the actual
   record and balance. An unscheduled step writes
   `observation_status=NotScheduled` and contains no balance value, including no
   synthetic zero.
3. It flushes and syncs every file, then syncs the partial directory. Any observer,
   encoding, hashing, or write failure removes the partial bundle and leaves both
   in-memory and durable frontiers at the prior committed clock.
4. The infallible prepared commit advances the in-memory state.
5. One rename publishes the complete bundle, followed by parent-directory sync.
   Only then does the durable frontier advance. A post-commit rename or sync failure
   retains the partial bundle, reports in-memory, durable, and provisional clocks
   separately, and stops qualification; it never claims crash-atomicity across
   memory and filesystem.

Rejected or numerical-error attempts retain their attempted interval and indicators
as separate attempt records and have no state snapshot. Scheduled bundles contain
state, attempt, record, and balance atomically; unscheduled bundles contain state,
attempt, and the explicit NotScheduled record atomically. Nine observer nodes remain
in the profile, but only the eight positive scheduled commits perform the expensive
observer.

## Verification

Five focused tests cover exact size/cap admission, command refusal, existing-output
refusal, hash/corruption detection, and the complete tiny stage/publish/validate/
cleanup transaction. Formatting and strict Clippy pass. The 434-line file has
maximum cyclomatic complexity 9, cognitive complexity 2, Halstead difficulty
19.25, 89.76% executable-line coverage, 80% branch coverage, and maximum CRAP
20 with no violation.

The decision is positive for a separately identified N384 experiment harness.
It does not justify a production checkpoint API or resume promise. A trajectory
launch should choose 128 GiB for at most 64 committed steps or 256 GiB for at most
128, refuse one byte short before numerical allocation, and retain only small
manifests in Git while storing bulk state files in the admitted artifact location.
