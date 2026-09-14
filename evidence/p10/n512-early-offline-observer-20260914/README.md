# P10 N512 early offline balance-observer preparation (2026-09-14, unexecuted)

Read-only **preparation** of OFFLINE BALANCE DIAGNOSTIC admission for the already
durable EARLY N512 clock-1536 captured state, so observation can proceed while
Sulaco integration continues. **No snapshot was loaded, no solver ran, no
network or SSH was used, and no numerical balance result is claimed here.**
This directory contains only small preparation helpers, one unexecuted manifest
template, refusal tests and notes. `qualification=false`, `resumable=false`;
this is an intermediate observation of a trajectory whose real endpoint remains
**4096** in the identity and in the frozen plans — never a completed trajectory
and never an accepted window.

## Inputs (verified byte-for-byte, never modified)

| input | SHA-256 |
| --- | --- |
| `work/early-observer-inputs/v3-launch-plan.json` | `4c14ee169cfbbb2a182d972633aba1292ed041878a9c4322e64a527f87d85da8` |
| `work/early-observer-inputs/clock1536-record.json` | `4fbb8c213a41a5e88eca4599953e3fa4cd5d4a2af8975abe6062b51a066d6376` |
| `evidence/p10/n512-m512-endpoint-prep-20260913/frozen-plan.json` (trajectory plan) | `6e8103a1937e3e31be8b147a936b843d4dc166877ef5de5540fb51429e672634` |
| reviewed observer crate | `evidence/p10/offline-captured-observer` (unmodified; hashes in `source-sha256.list`) |

The clock record is byte-identical to the copy already inside the reviewed
observer crate (`harness/clock1536-record.json`, `cmp` clean), so the identity
bytes used here are the same reviewed input the crate's arithmetic ledger uses. `tests/refusals.sh` reads that tracked fixture directly (`evidence/p10/offline-captured-observer/harness/clock1536-record.json`); the git-ignored `work/early-observer-inputs` copy is no longer referenced by the test driver.

## What was prepared

- `builder/build_balance_manifest.py` — derives the balance manifest ONLY from
  hash-verified reviewed inputs (record, frozen v3 launch plan) plus an
  **externally reviewed full-file SHA-256** and the record's
  **actual state SHA-256**. It refuses any drifted record/plan, missing hash,
  malformed hash or invented placeholder (see tests). The all-zero file-SHA
  placeholder is accepted only under `--unexecuted-placeholder` and only with
  the placeholder snapshot path `PENDING_ROOT_ARCHIVE_BINDING/state.bin`.
  Since the Astra review it also (a) verifies each input from ONE bounded
  1 MiB byte buffer — the SHA-256 and the JSON parse consume the same bytes,
  so a re-open cannot slip different content past the hash — and (b) publishes
  create-only atomically: content goes to a process-unique temporary file that
  is fsynced and attached with `link(2)`, so any existing path (replay,
  concurrent winner, symlink, hard link, or an alias of any input) is refused
  and no overwrite is possible.

## Astra defect fixes (same session, review rounds 2-3)

| defect | fix | test evidence |
| --- | --- | --- |
| hash-then-reopen TOCTOU on inputs | `read_bounded` + `bind_hash_and_parse`: hash and parse bind to one bounded byte buffer (oversized inputs refused) | `builder_unit.py` `test_bounded_buffer_cap`, `test_hash_parse_same_bytes` |
| overwrite publication (`write_text`) | `publish_new`: create-only atomic `link(2)` publication; refuses existing paths, symlinks, broken links, hard links, input aliases and mutually-aliased inputs | `builder_unit.py` publication tests + driver `builder-replay`, `builder-race` (exactly one winner), `builder-alias-{symlink,hardlink,input}`, `frozen-record-untouched`, `no-publication-debris` |
| post-`link(2)` durability failure reported as bare OSError (exit 1) | `publication_uncertain`: once `link(2)` has created the destination, a directory fsync/close or temporary-cleanup failure prints an explicit uncertainty report (published destination + failed operation + errno/strerror) and exits status 2, preserving the published bytes | `builder_unit.py` `test_publish_new_uncertainty_directory_fsync`, `test_publish_new_uncertainty_directory_close`, `test_publish_new_uncertainty_temporary_cleanup` |
| README wording: "29 checks" implied all were Rust decoder checks | corrected to the exact split: 21 real Rust `read_manifest` checks + 17 Python builder checks (38 total) | this section, `tests/refusals.status` |
- `manifests/balance-clock1536-unexecuted-template.json` — the generated
  template: elapsed 1536, target 8192, epoch 24, accepted steps 24, exact
  recorded identity (`source=e25f3816f83c...`), plan hash unchanged
  (`4c14ee16...`), domain 512 / lengths 1 / nu 1, frozen exact-v2 case
  `e1236f7b...`, CM + M512 integration force, q = 2^-20, original tolerances
  `[1e-5, 1e-4]`/`[1e-5, 1e-5]` and guard `{3.3, 48}`, h64 schedule prefix
  `[{0,1536),64]` through 1536.
- `tests/refusals.sh` + `tests/artifacts/` — 38 checks, 38 passed / 0 failed
  (`tests/refusals.status`). **21 of them are executed through the REAL
  reviewed Rust `read_manifest`** via the observer `preflight` CLI
  (arithmetic-only; that mode never opens the snapshot file and allocates no
  state); the other **17 exercise the Python builder's hash-binding,
  create-only publication and refusal gates** (including the embedded
  `tests/builder_unit.py` suite, 7 checks of its own, and frozen-record
  byte-identity/no-debris assertions after alias attacks).
- `preflight/` — authoritative arithmetic preflight of the template: status 0,
  exact ledger total **235,548,209,862 bytes** (matches the reviewed crate
  README's conservative upper bound for this identity), `fits=true` at the
  frozen plan's 256 GiB address-space limit, `fits=false` at total − 1,
  `qualification=false`, `accepted_windows=0`, clock
  `{exponent:-20, target:8192, elapsed:1536, remaining:6656}`.
  `preflight/observer-binary.sha256` binds the release binary
  (`cargo build --release --offline -j 2`, log in `tests/build.log`).

## Exact admission distinction (verified in code, no weakened schema)

- The fixed `MATCHED_M512_SPATIAL` contract is
  `snapshot-comparison-adapter/harness/src/m512_spatial.rs`: it requires
  `comparison_endpoint == 4096` (line 87), the full two-segment schedule and
  the N384↔N512 manifest pair. **Nothing here touches or weakens it** — the
  observer crate never includes or calls `m512_spatial.rs` or `compare.rs`.
- `decode::read_manifest` (the only admission the observer uses) treats
  `comparison_kind` purely as a profile-shape gate
  (`decode.rs:108-133`): `FORCE_RESOLUTION_DIAGNOSTIC` admits
  cox-matthews + integration force `[384;3]` or `[512;3]`, plus
  `comparison_endpoint == elapsed`, a gapless schedule reaching that endpoint,
  and hash-envelope/plan-hash checks. The M384-vs-M512 **pair** semantics of
  that kind live only in `compare.rs:validate_force_manifests` (adapter pair
  path), which the observer never invokes.
- The kind + prefix convention itself is precedented at early clocks by the
  reviewed N384 force-comparison manifests
  (`snapshot-comparison-adapter/review-force-clock1536/{left,right}-*.json`:
  `comparison_endpoint == elapsed == 1536`, schedule `[{0,1536),64]`, identity
  still carrying `endpoint=4096`), and the N512 `plan_sha256=4c14ee16...`
  binding follows `m512_spatial.rs:9` (`N512_PLAN`).
- Therefore using the `FORCE_RESOLUTION_DIAGNOSTIC` CM512 admission for a
  **single-snapshot balance-only** input is not a misrepresentation: the
  manifest claims only what is true (this snapshot's method/force-grid/clock
  shape), and the observer output is hard-wired
  `scope=balance-diagnostic-only`, `qualification=false`,
  `accepted_windows=0` with **no force-comparison claim** — it measures doubled
  N1024-grid conservative balances on one borrowed snapshot. No scientific
  blocker was found; no schema was weakened.

## Coefficient/file hash binding honesty

`write_snapshot` (`avx-scheduled-endpoint/harness/src/artifact.rs:202-239`)
shows a step record's `state_sha256` IS the coefficient-stream SHA-256 (the
same digest stored as the file trailer and re-derived by `decode.rs`), and the
reviewed N384 pair proves the equation in practice
(`review-force-clock1536/records/n384-m512-record.json` `state_sha256` =
`right-n384-m512.json` `coefficient_sha256` = `f335f6bd...`). So
`coefficient_sha256=6501282b8224...` in the template is the **actual recorded
state SHA**, not an invention. The whole-`state.bin` `file_sha256` is
unknowable without the archived file, so the template carries the all-zero
valid-hex placeholder and is named `-unexecuted-template`; `run` mode would
fail closed until root supplies the real value.

## What root does next (actual artifact binding)

1. Copy the archived clock-1536 `state.bin` (plus record.json) from Sulaco.
2. Verify locally: `sha256sum state.bin` (full-file) and confirm the state's
   embedded record/`state_sha256` equals `6501282b8224d19baf2cd68e201e5c6db7ffbf906c277d1a2ebb0495fbc3d509`.
3. Rebuild the manifest with the real values (refuses anything else):
   `python3 builder/build_balance_manifest.py --record <record.json> --plan
   ../n512-m512-endpoint-prep-20260913/proposed-launch/v3-launch-plan.json
   --output manifests/balance-clock1536-bound.json --snapshot <state.bin path>
   --file-sha256 <reviewed full-file sha>`.
4. Observe per the reviewed observer README (`preflight` then `run`, sample
   dimension 1024, workers 32 per the frozen plan's offline-observer geometry;
   Baccus actual-memory measurement still required before allocation — the
   ledger is a conservative upper bound, not a measured allocator peak).

## Honest limits

- `accepted_steps`/`epoch` are bound to the snapshot header only at
  `decode::load` time; the arithmetic `preflight` cannot check step counts
  against the schedule beyond the clock/prefix checks shown in tests. The
  builder mitigates this by deriving all four clock fields from the
  hash-frozen record.
- No N512 array was ever allocated here; the 235,548,209,862-byte figure is
  the reviewed crate's conservative arithmetic upper bound, matching its README.
- This preparation qualifies nothing: no window, no trajectory completion, no
  force comparison, no resume path (`resume=unsupported`).

## Root actual-artifact binding review (same session, completed)

Root delivered the durable bundle under
`/mnt/niva-array/nsbu-solver/work/n512-early-observer-artifacts-20260914/`
(pointer `work/early-observer-inputs/actual-capture-ready.json`,
`sha256=7d7252754ad9d6d995dff9bf26f68bd202f104d34d4a7fe1ad3dbe6331fc9b52`) and
requested an independent helper review. Verified here (read-only; `state.bin`
was streamed through SHA-256 exactly once and never decoded or loaded):

- bundle `clock1536/record.json` is **byte-identical** to the reviewed
  `work/early-observer-inputs/clock1536-record.json` (`cmp` clean, `4fbb8c21...`);
- independent full-file hash `sha256sum state.bin` =
  `7a4f05338c3cb7308a18c3788f2cc3a192fea9a93e3c1a07980b9963a3dbff55`,
  matching root's copy receipt and manifest; file size 3,233,809,650 equals
  the decoder's expected layout (12+8+1150 identity+64+3,233,808,384+32);
- root's manual `clock1536-balance-input.json` reviewed field-by-field against
  this helper's template: **every semantic field identical** (kind, identity,
  source/plan/coefficient hashes, clock/epoch/steps, q-20, prefix schedule
  `[{0,1536),64]`, force dims, tolerances, guard); differences are only the
  absolute snapshot path, the absolute plan path (verified to carry the same
  frozen `4c14ee16...` bytes) and the now-real file SHA;
- root's `clock1536-preflight.json` matches this helper's arithmetic ledger
  exactly (`total_bytes=235548209862`, clock `remaining=6656`, `fits=true`);
- helper-bound artifact `manifests/balance-clock1536-bound.json` (built by the
  hash-gated builder from the actual artifacts) then passed the real Rust
  `read_manifest` preflight: `preflight/balance-clock1536-bound-preflight.*`
  (status 0, `qualification=false`, `scope=balance-diagnostic-only`,
  `fits=true` at the frozen 256 GiB address-space limit).

The observation itself (`run` mode: snapshot decode plus ~219 GiB doubled-grid
allocation on Baccus per the frozen plan's geometry) remains root's executed
step with a fresh actual-memory measurement; nothing here simulates or pre-empts
it.

## Inventory repair (Astra review round 4)

- Removed `builder/__pycache__/build_balance_manifest.cpython-312.pyc`: a
  generated Python bytecode cache (git-ignored upstream) that the inventory
  had incorrectly listed.
- Removed `tests/artifacts/alias-symlink.json`: an absolute, checkout-specific
  symlink to this checkout's `v3-launch-plan.json` (recreated transiently by
  `tests/refusals.sh` during the `builder-alias-symlink` check, so it is
  deliberately absent from `SHA256SUMS`).
- `SHA256SUMS` regenerated from the remaining 112 files: `sha256sum -c`
  passes and the inventory lists exactly the packet's files (the
  `SHA256SUMS` file itself, as always, excluded). Test driver re-run after
  the repair: 38 passed / 0 failed; frozen reviewed inputs untouched.
