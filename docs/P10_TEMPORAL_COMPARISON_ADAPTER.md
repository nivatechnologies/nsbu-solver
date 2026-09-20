# P10 N512 temporal comparison adapter

`tools/prepare_temporal_comparison.py` prepares the pairwise comparison
manifest inputs for the N512/M512 h64/h32/h16 temporal family: three Cox--
Matthews trajectories on the identical N512 retained lattice with M512 force,
the same frozen case, quantum `-20`, target 8192, tolerances `[1e-5,1e-4]` and
`[1e-5,1e-5]`, advective guard 3.3, each evolved independently from exact rest
under its own nested piecewise schedule and captured at the eight shared
clocks 512, 1024, 1536, 2048, 2560, 3072, 3584 and 4096. The closed contract
lives in `tools/temporal_comparison_contract.py`. The adapter is read-only and
preparation-only: a successful run is comparison evidence only and cannot
itself accept a PDE window; no tolerance is applied or changed.

The nested family is fixed: `h64` runs 32 steps of 64 through 2048 and 16 steps
of 128 through 4096 (48 attempts, the reviewed v3 N512 anchor, identity,
profile, source and plan hash bound exactly); `h32` runs steps of 32 then 64
(96 attempts); `h16` runs steps of 16 then 32 (192 attempts). Every coarse
node is a fine node in both slabs, so the comparison pairs are exactly
`h64/h32` and `h32/h16`. The finer branches carry explicit `PENDING_*`
bindings until their real capture identities and plans are reviewed; strict
admission refuses any pending binding.

## Exact clocks, epochs and capture names

For each branch and shared clock the adapter derives the accepted-attempt count
from the schedule (for example h64 at 1536 is epoch 24; h32 at 2560 is epoch
72; h16 at 4096 is epoch 192) and the transactional bundle name
`step-{epoch:03}-clock-{clock:04}` from
[`step_artifact.rs`](../evidence/p10/avx-scheduled-endpoint/harness/src/step_artifact.rs).
A capture is admitted only when its record matches the derived clock, epoch and
accepted-step count exactly, names the branch's immutable identity, carries
`resumable=false`, `qualification=false`, `observation_status=
CapturedActualState`, `offline_observer_node=true`, the branch observer
execution and the bound coefficient payload of exactly 3,233,808,384 bytes
(`512 * 512 * 257` half-spectrum coefficients times three components times 16
bytes). Any earlier refused or replayed attempt breaks the name/epoch algebra
and is refused.

Each emitted manifest side carries the branch schedule truncated at the shared
clock, so the frozen decoder in
[`compare.rs`](../evidence/p10/snapshot-comparison-adapter/harness/src/compare.rs)
derives `epoch == accepted_steps == schedule_steps` at that clock while the
from-rest evolution up to the clock is unchanged; the trajectory endpoint 4096
stays bound separately in the bookkeeping and is never relabelled.

## Inputs

- `--family-plan`: the emitted family plan (`family-plan.json` in the
  [adapter packet](../evidence/p10/n512-temporal-comparison-adapter-20260914/README.md))
  with every pending binding replaced by the reviewed concrete values.
- `--root`: the capture root holding `<branch>/<bundle>/record.json` and
  `state.bin`, each branch's capture plan file (hashed against its bound
  `plan_sha256`), `state-inventory.json`
  (`p10-n512-temporal-file-inventory-v1`) carrying the synced whole-file
  SHA-256 of exactly the 24 shared-clock payloads, and one arithmetic binding
  per nested pair (`p10-n512-temporal-arithmetic-binding-v1`) carrying the
  reviewed serial/W3 lineage whose left/right sides name the pair's actual
  sources, backend, execution and unrelabelled profiles, plus its hash-checked
  evidence file.

Time diagnostics require that arithmetic-control binding on both sides; the
adapter refuses to fabricate it. Each per-pair evidence file is parsed during
preparation and must be JSON whose content matches the embedded
ArithmeticReview exactly; it is then materialized create-only as
`arithmetic-evidence.json` inside every pair-clock directory, exactly where the
emitted manifest resolves it, and remains bound by `evidence_sha256`. The
decoder (`decode::read_manifest`) admits TIME_DIAGNOSTIC evolution for
cox--matthews with M384 or M512 integration-force dimensions, refuses every
other force dimension, and does not itself bind the retained lattice; the
comparator (`compare::admitted_time_force_dimensions`) then admits
`TIME_DIAGNOSTIC` pairs beside the reviewed M384 time family only for the exact
M512 family, i.e. M512 force on the exact N512 lattice, refusing M512
evolution on any other grid.

## Refusals and independence

Refusals (exit 1, one `refused:` line, nothing published) cover: missing or
renamed bundles, records, payloads or plan files; duplicate JSON keys;
non-finite or non-hex values; clock, epoch, accepted-step, identity, observer
or payload mismatches; branch-identical or repeated coefficient/whole-file
hashes (independent evolution must be distinguishable); hash-layer collisions;
identity relabelling, pending bindings, tolerance, guard, case, schedule
(including non-integer or boolean tick values), clock-set or
arithmetic-constant drift from the closed contract; wrong pair
ordering; and lineage/evidence conflicts. Every check completes before any
write; outputs are then published create-only with hard links and fsync, and
an existing output is never overwritten (exit 2 marks uncertain publication
while retaining the destination).

The adapter never opens a `state.bin` payload: coefficient and whole-file
hashes are carried through from the capture record and the synced inventory,
provenance that the bookkeeping labels explicitly. Streaming re-verification
remains the job of the capture preparation tool. Nothing is injected, reset or
reference-substituted: each side keeps its own identity, schedule, epochs and
hashes, and tests confirm the payloads can even be made unreadable while the
adapter still completes, and that the capture root is byte-identical after a
run.

## Disk arithmetic

The bound payload is 3,233,808,384 bytes per state. Bookkeeping binds: 24
payloads totalling 77,611,401,216 bytes; a per-branch state-file size of
`116 + identity_bytes + 3,233,808,384`; and the decoder reservation per pair of
`2 * 3,233,808,384 + 1,048,576 = 6,468,665,344` bytes, so all sixteen pair
invocations together reserve 103,498,645,504 bytes.

## Interface

```text
python3 tools/prepare_temporal_comparison.py --help
python3 tools/prepare_temporal_comparison.py \
  --emit-family-plan --output /review/n512-temporal-family-plan.json
python3 tools/prepare_temporal_comparison.py \
  --family-plan /review/n512-temporal-family-plan.json \
  --root /capture/n512-temporal-family \
  --output /comparison/n512-temporal-family
```

The output holds `pairs/<coarse>--<fine>/clock<C>/<branch>.json` manifest
inputs whose `snapshot` and `plan` bindings are paths relative to each pair
directory (they bind the capture root location), plus `bookkeeping.json` with
the clock-ascending ledger, exact bundle/epoch table, per-pair reservations,
plan/binding hashes and the explicit `not_assessed` acceptance record.

## Limits

No h32/h16 N512 capture, inventory or arithmetic review exists yet, so no real
comparison has been prepared; the emitted plan still carries its pending
bindings. Passing these tests prepares inputs only. Window qualification
follows the package's exit evidence per
[NEXT_CONCENTRATING_WINDOW.md](NEXT_CONCENTRATING_WINDOW.md), and zero
concentrating windows are accepted.
