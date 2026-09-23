# P10 snapshot manifest preparation

`tools/prepare_snapshot_manifest.py` prepares one side of an N512 endpoint
snapshot comparison: it binds a captured `state.bin` and its `record.json`
into an explicitly supplied reviewed comparison-input template and publishes
the result create-only. Validation primitives live in
`tools/prepare_snapshot_manifest_validation.py` and the closed scientific,
record and plan contract (including the endpoint template generator) in
`tools.prepare_snapshot_manifest_contract.py`. It prepares artifact bindings.
It does not accept numerical results, qualify a window or change any acceptance
criterion. No PDE endpoint is accepted by this tool, and no run using its output
is qualified until the package's own exit evidence exists.

The comparison template's `plan_sha256` binds the reviewed v3 launch plan
(`p10-n512-m512-v3-launch-plan-v1`, `4c14ee169cfbbb2a182d972633aba1292ed041878a9c4322e64a527f87d85da8`).
That launch plan carries `profile` as a string, `resources` and `guards`, but no
numerical evolution metadata. The numerical contract (method, endpoint, clock
target, tolerances, attempt bound, advective limit, offline observer host) is
therefore taken from the separately reviewed trajectory capture plan
(`p10-n512-m512-endpoint-capture-plan-v1`, `frozen-plan.json`,
`6e8103a1937e3e31be8b147a936b843d4dc166877ef5de5540fb51429e672634`), which must
be supplied explicitly with `--trajectory-plan`; the tool refuses unless that
file's SHA-256 equals the launch plan's
`historical_preparation.immutable_trajectory_plan_sha256`. No field is inferred
and no absent field is invented.

## Why preparation is not acceptance

Preparation answers "which captured artifact is this manifest bound to, and do
the captured bytes match the externally reviewed hashes". It never blesses a
candidate: the operator supplies the expected whole-file SHA-256 and the
expected launch-plan SHA-256 explicitly on the command line, and the tool refuses
unless computed hashes match. Computed values are never silently copied into
expected bindings. Endpoint admission follows the frozen comparison contract:
the comparison adapter's decoder fixes `comparison_endpoint` in the reviewed
template, and the tool refuses any captured record whose clock, epoch or
accepted-step count does not equal that frozen endpoint and the reviewed
template, so intermediate clocks are not admitted here. The early
`clock=1536;epoch=24;accepted_steps=24` observer record is a captured
intermediate state, not endpoint data, and is refused as incomplete for the
4096-tick endpoint.

## Inputs and contract sources

The binary layout follows the transactional writer evidence in
[`artifact/mod.rs`](../evidence/p10/avx-scheduled-endpoint/harness/src/artifact/mod.rs)
and [`step_artifact.rs`](../evidence/p10/avx-scheduled-endpoint/harness/src/step_artifact.rs):
magic `P10AVXSNAP1\0`, u64 identity length, identity bytes, four little-endian
u128 header words (elapsed, target, epoch, accepted steps), three half-spectrum
component payloads and a 32-byte SHA-256 coefficient trailer, with no trailing
bytes. The captured record schema is `p10-avx-n512-observer-state-v1` with
`observation_status` `CapturedActualState`, `resumable` false,
`qualification` false and the expected offline observer metadata
(`offline_observer_node` true and an `observer_execution` string naming the
capture plan's offline observer host). The manifest envelope, evolution and
schedule checks mirror the frozen decoder in
[`decode.rs`](../evidence/p10/snapshot-comparison-adapter/harness/src/decode.rs),
the half-spectrum size follows
[`layout.rs`](../crates/nsbu-solver/src/domain/layout.rs), and the reviewed
endpoint contract matches
[`m512_spatial.rs`](../evidence/p10/snapshot-comparison-adapter/harness/src/m512_spatial.rs).
The template is a reviewed input: scientific and provenance fields (identity,
source commit, plan hash, evolution, clocks) must already be correct. Only the
four artifact bindings `snapshot`, `plan`, `coefficient_sha256` and
`file_sha256` may carry the placeholders `PENDING_SNAPSHOT_BINDING`,
`PENDING_PLAN_BINDING`, `PENDING_COEFFICIENT_SHA256` and `PENDING_FILE_SHA256`;
the tool fills a placeholder only after validation and refuses any concrete
binding that conflicts with the captured artifact. The template must also be
the endpoint profile: the `snapshot` and `plan` bindings (placeholder or plain
relative names) must resolve to the supplied files beside the output manifest.

## Interface

```text
python3 tools/prepare_snapshot_manifest.py --help
```

The reviewed endpoint comparison template itself is generated (create-only) from
the explicit contract constants — target 8192, endpoint 4096, epoch and accepted
steps 48, exact sources and profile, and the closed
[`m512_spatial.rs`](../evidence/p10/snapshot-comparison-adapter/harness/src/m512_spatial.rs)
guard `[3.3, 48]` — leaving the endpoint `coefficient_sha256` and `file_sha256`
as explicit `PENDING_*` placeholders:

```text
python3 tools/prepare_snapshot_manifest.py \
  --emit-endpoint-template --output /review/endpoint4096-n512-template.json
```

A prepared manifest appears only after every check passes, written to a
temporary file, fsynced and hard-linked into place; a refusal raised before
staging creates nothing and an existing output is never overwritten. A refusal
prints one `refused: <reason>` line to stderr and exits 1; success exits 0.
Every fallible step after the destination link succeeds — removing the staged
copy, opening the directory, and the directory fsync — is reported as an
uncertain publication, never a plain refusal: the tool prints
`publication uncertain: <reason>` and exits 2, always retaining the linked
destination and naming the exact remaining staged-copy state so the operator is
never told nothing was published when the destination exists. Input JSON is
bounded (64 KiB template, 8 KiB record, 1 MiB launch and trajectory plan,
16 KiB identity); each bounded read consumes at most the byte bound plus one
byte before parsing or hashing, so an oversized file is refused without
allocating its full size. Each launch plan and trajectory plan is read once and
its bytes are hashed and parsed from that single read. Input JSON also rejects
duplicate keys, non-finite numbers, invalid constants and unknown fields, and
the binary is streamed in fixed 1 MiB chunks with a preflight exact file-size
check. There is no checkpoint or resume capability; a refused attempt is re-run
from scratch. Example (paths are illustrative; hashes come from your reviewed
external record):

```text
python3 tools/prepare_snapshot_manifest.py \
  --template /review/endpoint4096-n512-template.json \
  --bundle /capture/step-048-clock-4096 \
  --plan /review/v3-launch-plan.json \
  --trajectory-plan /review/frozen-plan.json \
  --expected-file-sha256 <reviewed 64-hex> \
  --expected-plan-sha256 <reviewed v3-launch-plan 64-hex> \
  --output /comparison/endpoint4096-n512-input.json
```

## Scope limits

This tool qualifies nothing. It does not launch, resume or repair a capture,
does not evaluate tolerances, and does not interpret comparison outputs.
The tests live in four files, each under 500 lines: shared builders in
`snapshot_manifest_fixtures.py`, CLI/IO/publication behaviour in
`test_prepare_snapshot_manifest.py`, the closed contract in
`test_prepare_snapshot_manifest_contract.py`, and a parameterized refusal matrix
in `test_prepare_snapshot_manifest_refusals.py`. They build tiny synthetic
`state.bin` and independent state fixtures in a scratch directory, independent
of any real capture; they exercise the generic validator at the small grid the
CLI profile admits and cover the positive baseline plus changed-file-hash,
changed-header, changed-record, wrong external plan hash, unbound trajectory
hash, wrong launch schema, truncation, trailing-byte, magic, snapshot
identity-length, identity-bytes and coefficient-trailer header tampers,
conflicting-template, plan-name-conflict and plan-not-beside-output binding,
duplicate-key, non-finite, non-string comparison_kind, JSON nesting past the
32-level bound (multi-KiB deep nests included) as one-line refusals,
oversized-plan and
oversized-trajectory bounded-read, single-read trajectory proof, existing-output
preservation and publication-uncertainty refusals for the link conflict, the
staged-copy unlink after link, the directory open after link, the directory
fsync, the directory close after link (alone and combined with the fsync, both
failure contexts preserved) and the staged-file fsync, plus the link-race
refusal and clean stage-failure removal. A parameterized matrix of 100 refusals
then tampers launch-plan, capture-plan, evolution, template and record fields,
mostly over the ACTUAL reviewed bytes. They also run the ACTUAL reviewed bytes: the repository v3 launch plan is
admitted against the reviewed trajectory capture plan and the closed-contract
endpoint template, tampering its authorization, executed-attempt or qualification
fields is refused, and the actual early `clock1536` record, reproduced
byte-for-byte as a hash-pinned self-contained literal so fresh checkouts need no
ignored `work/` inputs, is confirmed refused as incomplete for the endpoint. The generated endpoint template is checked for
exact Rust-decoder field-set, two-element tolerance, and
domain/execution/profile/source/guard binding, and is passed through the built
adapter `read_manifest` with the placeholder hashes substituted by dummy valid
hex and the real reviewed plan beside it: the decoder admits the manifest and
only the later pair-stage contract check refuses (SCHEMA-ONLY admission, no
capture or payload proof; a broken plan hash and a broken evolution confirm the
probe detects real rejections). N512 captures do exist remotely on Sulaco; what
is absent is execution evidence for this tool against a real capture: no
prepared manifest has been produced from those artifacts here (the only local
N512 record is the incomplete `clock1536` state), window qualification still
follows the package's exit evidence per
[NEXT_CONCENTRATING_WINDOW.md](NEXT_CONCENTRATING_WINDOW.md), and zero
concentrating windows are accepted.
