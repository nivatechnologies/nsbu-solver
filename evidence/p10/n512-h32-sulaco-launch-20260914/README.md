# N512/M512 temporal-h32 SULACO capture launch candidate (r6: zero-Any/unknown typing gate closed, evidence re-verified; r5: leak-census-hardened, re-inventoried)

Prepared candidate for a later root-supervised launch on **sulaco**. This packet
does **not** launch, deploy, SSH, allocate an N512 field, or commit the solver
run, and it does not modify any inert stage. r5 is a fresh, separate Sulaco
stage created by re-freezing `scripts/freeze_sulaco_stage.py` after the final
source re-inventory (r4 is archived and off the canonical path); it closes every
item of the Astra interim review of the Baccus h32 candidate and the r4 test
process-leak finding. This revision additionally closes the final Astra
review's packet-evidence findings — complete SHA256SUMS manifest, metric/typing
scope precision, runner-kill exit-code and simulation attributions, repeat
counting. The r6 revision additionally closes the retained typing gate at zero:
`pyrightconfig.json` now runs `reportAny`/`reportExplicitAny`/
`reportUnknownMemberType` all `error` with **zero diagnostics**, achieved by real
repairs — typed `argparse.Namespace` subclasses, direct ctypes `prctl` result
comparisons, and a new shared validated-JSON boundary module
`scripts/h32_json.py` (18th packet script) — not by ignores, suppressions or
blanket casts. The Python launch-tool changes are confined to typed argument,
JSON, regex and ctypes boundaries; the stage, Rust sources/source inventory,
watchdog bytes and numerical runtime contract are untouched. The Python unit,
selftest, preflight and census transcripts were regenerated against the current
bytes. The four Rust harness test/Clippy transcripts are reused unchanged from
r5 because all 610 numerical inputs and the staged binary are unchanged; they
are source-bound reuse, not claims of a fresh Rust execution in r6.
`qualification=false` and `accepted_windows=0` everywhere here; this is a
proposed, inert, root-gated candidate, not a qualification.

## Test process-leak census (r5 — external VERIFIED subreaper)

The census process enables and **VERIFIES** `PR_SET_CHILD_SUBREAPER` before any
job starts, so every descendant an escaping test leaves behind is adopted
directly by the census and the descendant-reachability attribution is decisive,
not heuristic. Every `/proc` state is counted **including `Z`** (a zombie is an
unreaped leak); output goes to files (never a pipe an escaped descendant can
hold open); the external sweep is strictly post-snapshot; foreign processes are
never attributed (only pids absent from the per-job pre-snapshot **and**
reparented under the census count). The sealed r5 run preceded publication of
`h32-census-complete.marker`. The current-byte r6 rerun occurred after that
marker existed, under a separately verified exclusive lifecycle-test lane; the
marker is historical evidence, not a lock. Census digest (verbatim script
summary lines plus clearly-scoped editorial notes): `docs/leak-census.stdout`;
per-job JSONL records live under the census `--out` directories.

- **Leak census** (`scripts/leak_census.py --repeats 2`): every one of the 152
  unit-test methods plus the FULL suite plus the guarded selftest, twice =
  **308 jobs, `bad=0`** (`r0=154` and `r1=154` are two complete repeats of the
  same 154-job set — 152 methods + FULL-suite + SELFTEST per repeat),
   `subreaper=VERIFIED zombie_states_counted=True`, and **final external
   live/Z/unconfirmed = 0** (zero leaks in either pass, zero unconfirmed after
   sweep). The r6 current-byte re-run reproduced this exactly: a second
   `CENSUS DONE jobs=308 bad=0 subreaper=VERIFIED zombie_states_counted=True`
   with `r0=154`/`r1=154`, zero leaks, zero unconfirmed.
- **Runner-violence census** (`scripts/runner_kill_census.py`, SIGKILL of the
  runner mid-method at 0.8/1.6/3.0 s): **48 probes** (16 spawn-capable methods
   × 3 delays), `subreaper=VERIFIED`. The intentional leader-death stranded
   **18 pre-sweep survivor probe-records / 40 descendant instances** (per the
   authoritative sealed JSONL snapshot states: **26 live + 14 zombies**), every one packet-owned
   and reparented to the census subreaper. Survivor-producing probe classes,
   exactly (sealed JSONL): `test_h32_children.TeardownPathTests` (15 records),
   `test_h32_owner.TerminateOwnedTests` (2) and
   `test_h32_children.InterruptionTests` (1) — no other class produced any
   survivor. All 40 instances are **attributed and confirmed swept**
   (each of the 18 records shows post-cleanup `unconfirmed=[]` in the census
   stdout), and an independent external `/proc` sweep of all attributed pids
   re-run during the r6 evidence pass returns **0 live / 0 zombie → final
   external sweep zero**.
   Current-byte r6 re-runs (five fresh attempts) returned `stranded=17`
   (38 instances each; runs 1 and 3 recorded 27 live + 11 zombies, while runs
   2, 4 and 5 recorded 26 live + 12 zombies; same three classes, 14/2/1): the only
   delta is the 3.0 s kill probe missing one fixture-boundary stranding window
   — a kill-timing diagnostic on the current bytes, not a behavior change; the
   per-probe `unconfirmed=[]` and external-sweep-zero results hold identically
   (both runs recorded verbatim in `docs/leak-census.stdout`).
   `runner_kill_census.py` exits **1 whenever any pre-sweep survivor record
   appeared at all**, so this run exited **1 by design** (the 18 stranded
   records; 17 on current bytes) even though cleanup succeeded — the exit code
   is the intentional strand diagnostic, not a cleanup failure, and no rc 0 is
   claimed for it.

## Current-byte build / quality gates (authoritative, current bytes)

- Rust (source-bound r5 transcripts reused unchanged in r6):
  `n512-m512-temporal-h32` **32 pass**, default **18 pass**,
  `n512-m512-piecewise-cadv33` **28 pass**, Rust-written fixture regenerates
  **byte-identical**, Clippy **clean** (`-D warnings`).
- Python: **152 unit tests PASS** and the guarded selftest **PASS** (release /
  abort / re-arm / decision-uncertain cases).
- Coverage (production scope, regenerated on current bytes): **88.6714 %
  lines** (2129/2401), **80.4136 % actual branches** (`covered_branches 661 /
  num_branches 822` in the coverage JSON — the branch denominator already
  excludes partial branches; not re-subtracted). Measured by `coverage run
  --branch` over the 152-test suite, then `--append`-ing the guarded selftest
  run (the selftest imports `h32_run` in-process), same method as r5.
- Typing (exact scope): `mypy --strict --disallow-any-explicit` passes on all
  **18/18** packet scripts, and **basedpyright 1.40.0 reports 0 diagnostics**
  under this packet's `pyrightconfig.json` with strict mode and
  `reportAny="error"`, `reportExplicitAny="error"` and
  `reportUnknownMemberType="error"` all retained — the retained zero-`Any`/
  zero-`unknown` gate, closed by real repairs (typed `argparse.Namespace`
  subclasses for `parse_args`, a validated-JSON boundary module
  `scripts/h32_json.py` with duplicate-key rejection at the packet readers
  whose contracts require unique keys,
  `object`-typed JSON traversals narrowed by `isinstance`, and direct ctypes
  `prctl` return comparisons) with no ignores, suppressions, `# type: ignore`
  comments or untyped wrappers. Scope remains `scripts/` (the packet's declared
  production boundary); the claim is zero diagnostics under these exact
  settings, not a universal statement about every tool.
- Complexity (radon/complexipy over packet scripts+tests, current bytes):
  cyclomatic **CC max 21**, **cognitive max 19**; the enforced gate is Halstead
  **difficulty**, max **9.20886** (`scripts/h32_launch_supervisor.py`) against
  the **< 80 difficulty** policy — Halstead **volume is not a packet policy**
  (observed volume max **2104.35572**, `tests/test_h32_snapshot.py`,
  descriptive only); the crap gate (scripts-only scope, as disclosed) produced
  **58 function rows, all `scripts/`-only with zero test-file rows**, so
  **CRAP max 20.469 is scripts-only, not test-inclusive**; the test-inclusive
  measurement against the same coverage data additionally shows two
  test-harness rows ≥ 25 (`tests/process_guard.py::kill_and_reap_remaining`
  44.30, `tests/test_h32_snapshot.py::layout_expectations` 37.14 — unexercised
  error-handling branches in harness code), disclosed here rather than counted
  as a production result; largest packet file **497 lines**
  (`tests/test_h32_children.py`; harness largest `src/main.rs` 438). Every
  production/test/helper file stays < 500 lines.

## Binary / source reproduction (r5)

- `RUN_SOURCE = fa441601bee855a7c67b2a8f1f29e3a5a2a123b7e3e3a6ca3417b3f294ec6d55`
  = SHA-256 over the sorted **610-entry** hashed source inventory
  (`docs/source-sha256.list`; recomputed deterministically by
  `scripts/source_inventory.py`). It is the exact `source=` field of the
  production profile identity.
- Staged solver binary `sha256 = 3b9ccff93d41200d664aadcfbec32a406e139df90c06cca989bda5a518721de4`,
  byte-identical to `harness/target/release/p10-avx-scheduled-endpoint`. A fresh
  original-tree build **and** a mount-namespace cleanroom build, both under
  `--remap-path-prefix` and `--locked --offline
  --features n512-m512-temporal-h32`, each reproduce exactly this hash from the
  exact `RUN_SOURCE`.
- Path-identity note (truthful): an ordinary build at the repo path, and a naive
  cleanroom build that did not remap the source/cargo path prefix, produce
  *different* binary hashes. Those are known **path-identity diagnostics**
  (embedded build path), **not** accepted builds; the accepted staged identity is
  the `--remap-path-prefix`-reproducible `3b9ccff9`.

## Astra interim-review items and how each is closed

| blocker | closure |
|---|---|
| no producer-side pause; attempt 2 could start before validation | new opt-in Rust barrier (`harness/src/barrier/`): after the durable clock32 commit the solver publishes a create-only armed receipt, then parks until it authenticates a release or abort token; on abort/expiry it exits before attempt 2. The barrier is an explicit one-shot state machine (`Waiting/Armed/Completed/Disarmed`): after one authenticated release at clock 32 it is permanently `Completed` and can never re-arm or park again (unit-proven; a re-entrant `after_commit` while `Armed` is a hard error) |
| pause forgeable | token = `SHA256(secret‖action‖nonce‖clock‖state_sha‖secret)`; the 256-bit secret travels only in the spawn environment and the token is bound to nonce + committed state hash; cross-language vector test in both Rust and Python suites |
| counters/ratios not enforced | release requires stdout AND durable `attempt.json` exact 12 RHS / 7 hits / 5 misses / 0 steady, finite local error ratios ≤ 1.0 (distinct from the advective 3.3 acceptance limit), integration inside the measured ceiling |
| identity fields unchecked | `attempt.json`/`record.json` require exact schema, clock 32, epoch 1, `accepted_steps=1`, `attempted_from=0→attempted_to=32`, `ticks=32`, coefficient_bytes 3,233,808,384, exact snapshot size, `resumable=false`, `qualification=false`; identity is parsed as `key=value` fields joined by `;` and every required field (`source`, `profile`, `schedule`, `endpoint=4096`, `attempt_schema`, `resume=unsupported`, `method=cox-matthews`) is compared by **whole-field equality** — the adversarial `endpoint=40960` / `endpoint4096` variants are rejected (test-proven) where the old substring check accepted them; duplicated/unknown identity fields, duplicate JSON keys and partial siblings are malformed |
| snapshot payload only size-checked | `state.bin` is validated against the byte layout **derived from the Rust writer** (`artifact/snapshot.rs::write_snapshot`): magic `P10AVXSNAP1\0`, u64 LE identity length, identity bytes, four u128 LE clock words (elapsed/target/epoch/accepted_steps) at their real offsets checked against the exact expected sequence, exact total size, SHA-256 over the actual coefficient payload, and the trailing digest over exactly what the Rust writer digests; sparse (hole-detected via `st_blocks`, all-zero, optional NUL-run), truncated, padded, payload-corrupted and wrong-identity files are refused; the real Rust-written tiny fixture (`tests/fixtures/n512-h32-snapshot-minimal.bin` + `snapshot-layout.json`, identity document at `docs/identity-expected.json`) is consumed by `test_h32_snapshot.RustWriterFixtureTests` and regenerates **byte-identically** from the current writer via the `NSBU_SNAPSHOT_FIXTURE_OUT`/`NSBU_SNAPSHOT_LAYOUT_OUT`/`NSBU_IDENTITY_DOC_OUT` fixture test |
| measured resources unchecked | decision receipt records sampled `MemAvailable`, solver RSS vs the 207,627,647,760 cap, and remaining disk vs the 95-bundle floor; any miss writes the abort token |
| explicit release/abort | supervisor issues **at most one** create-only token per attempt; pre-existing token bytes are never overwritten (`uncertain_exists`). Barrier one-shot on both sides: the Rust barrier is a `Waiting→Armed→Completed/Disarmed` state machine that can never re-arm after the authenticated clock-32 release, and the supervisor treats a second armed line, an armed line at any other clock, or any release/abort token already on disk at decision time as a protocol violation → create-only failure receipt + abort/teardown with no further token (selftest `rearm` case and unit tests) |
| signals could strand unregistered children | `deferred_signals()` blocks SIGINT/SIGTERM across every Popen+registration window; child pre-exec restores the intended mask. The watchdog script bytes are sha256-pinned against `watchdog_sha256` immediately before spawn (mismatch = refuse). BOTH children (watchdog and solver) have their pid/pgid/starttime/cmdline identity frozen at spawn, and success, error and SIGINT/SIGTERM paths all run the same terminate+reap teardown: per-`/proc` immutable identity re-check before any signal (a changed identity gets no group signal, at most a direct-child terminate), TERM→KILL escalation after the pinned 60 s grace, full group drain, and a create-only failure receipt whenever a member cannot be confirmed reaped or identity drifted (real-process tests incl. a TERM-ignoring grandchild and a forged-identity live process that receives no signal) |
| descendants survive leader exit | `drain_group` TERMs every live `/proc` group member (zombies skipped, protected PIDs never), KILLs survivors after the pinned grace (selftest/real-process test uses a TERM-ignoring grandchild), then always reaps both direct children, even when the leader already exited (real-process test) |
| receipts not create-only/no uncertainty | `h32_receipts` helper: every token/receipt is written `O_CREAT|O_EXCL` with fsync of the file AND the containing directory; EEXIST → `uncertain_exists`, never overwritten. A per-attempt `ReceiptLedger` makes any non-`written` status sticky: before the release token and before the completion/success receipt the supervisor re-checks (a) zero ledger uncertainty, (b) the solver child is alive with its pinned identity, (c) for success, the first-step bundle revalidates, the terminal state and capture set are complete and every child is confirmed reaped; any doubt → refuse + create-only failure receipt (selftest + unit tests) |
| preflight could read the wrong filesystem / zero memory | `MemAvailable == 0` is refused as a broken reading (`mem_available_broken_reading(0)`), never treated as free memory and never released; the disk `statvfs` runs on the nearest existing ancestor of the **actual output path** (`resolve_probe_path`, test asserts the probe on a tmpfs output dir rather than cwd/stage); the first-step wall/integration timer starts **before** `Popen`, so spawn latency counts (`first_step_elapsed_seconds` in the decision receipt; selftest asserts driver-measured elapsed ≥ the fake solver's own spawn-to-first-step latency, and a unit test asserts the timer precedes the spawn call) |

## Fresh Sulaco stage (create-new, inert; Baccus stage untouched)

`/mnt/niva-array/nsbu-solver/work/n512-h32-sulaco-capture-20260914-r5/`:
`frozen-plan.json` (sha256 `ac63c8c71e958d9a0b8e246338a1cccc09373eb9dde9084ea9012a87daa5acb9`,
frozen 2026-09-20T16:06:07Z by `scripts/freeze_sulaco_stage.py`; mirrored at
`docs/frozen-plan.json`), the release `solver`
(`3b9ccff93d41200d664aadcfbec32a406e139df90c06cca989bda5a518721de4`, byte-identical
to `harness/target/release/p10-avx-scheduled-endpoint` built from the inventoried
sources with path remap), `pgid-watchdog-v3.sh`
(`23f5a485c62fb764f0ba933a4d2bb1d524df4719d994e8d400ba23bf310f1c69`, unchanged
reviewed logic), `preflight.stdout`
(`479b0d4005418bb607498b3ebba85cae8ba304ce41de8ab787c5dca9fe19695c`, `total=cap`),
empty `logs/`, empty `barrier/`, no output/receipt.

| field | value | source |
|---|---|---|
| host | `sulaco` (actual `socket.gethostname()` gate; refusal proven from baccus) | `docs/preflight-only-baccus-host-refusal.stdout` |
| source identity | `RUN_SOURCE=fa441601bee855a7c67b2a8f1f29e3a5a2a123b7e3e3a6ca3417b3f294ec6d55` (610 hashed inputs) | `docs/source-sha256.list` (hash of every hashed input byte; `scripts/source_inventory.py`; tree uncommitted — no commit allowed this session) |
| profile / schedule / schemas | `n512-m512-h32to2048-h64to4096-cadv33-w3-pfft1ed6995`; `h32-clocks0-through2048-then-h64-through4096`; h32 observer/attempt/terminal schemas | harness preflight @ `3b9ccff9`, `docs/frozen-plan.json` |
| endpoint / attempts / limit | `endpoint=4096`, `maximum_attempts=96`, `advective_limit=3.3`, first bundle `step-001-clock-0032`, record epoch 1, 96 committed states | harness preflight, oracle-96 + `schedule.rs` |
| independent-from-rest / transactional | `from_rest=true`, `resume=unsupported` (comparison trajectories evolve independently from rest; analytical references never reset integrated state), create-only logs/receipts, no re-launch against existing output | `docs/frozen-plan.json`, `h32_contract.py`, selftest + unit tests |
| exact peak / memory floor | 207,627,647,760 → floor 241,987,386,128 (exact peak + 32 GiB; ≥ the required 241,987,189,520) | this stage's `preflight.stdout` |
| AS limit | 274,877,906,944 (256 GiB, unchanged reviewed value) | reviewed prep rule |
| disk | bound 310,453,075,968 (=96×(3,233,808,384+77,824)); floor 344,812,814,336 (bound + 32 GiB same-fs rule) | `step_artifact.rs` disk_preflight |
| first-step budgets | integration ≤ 1910 s, wall ≤ 2103 s = ceil(2×954.894656332), ceil(2×1051.20) | measured Sulaco N512 attempt only (`evidence/p10/n512-m512-sulaco-timing-20260913`); Baccus timings deliberately not used |
| deadline | ONE absolute deadline: frozen 2026-09-20T16:06:07Z + 48 h → 1790093167 (2026-09-22T16:06:07Z), `proposed_for_root`; the same epoch is the watchdog deadline, the barrier deadline cap and (via `min(wall, deadline-now)`) the single first-step/release gate | r5 `frozen-plan.json`; fresh re-freeze rather than extend silently |

## Preserved behaviour

From-rest integration only, exact immutable clocks, no analytical reset and no
resume (`resume=unsupported`; re-launch against existing output/receipt/logs
refuses), ownership only via confirmed fresh-session PID/PGID/starttime/cmdline,
protected PIDs never bound or signalled, create-only logs/receipts, pinned
watchdog poll 5 s / grace 60 s, terminal completeness = 96 committed bundles
with exact names plus the terminal line. The h32→h64 schedule (h32 clocks
0-through-2048, then h64 through 4096, endpoint 4096, 96 attempts) is the
compiled `n512-m512-temporal-h32` profile and is unchanged.

## Focused commands

The Python commands below regenerated the r6 Python transcripts. The Rust
commands identify the source-bound r5 transcripts reused unchanged for r6.

```
cd evidence/p10/n512-h32-sulaco-launch-20260914
basedpyright                                     # 0 diagnostics: strict + reportAny/
                                                 # reportExplicitAny/reportUnknownMemberType all "error"
mypy --strict --disallow-any-explicit scripts/*.py   # 18/18 scripts clean
cd evidence/p10/n512-h32-sulaco-launch-20260914/tests
python3 -m unittest discover -p 'test_*.py' -v   # 152 pass (incl. fixture tests)
python3 selftest_guarded_launch.py               # SELFTEST PASS (release/abort/rearm/uncertain)
cd .. && NSBU_RUN_N512_TEMPORAL_CAPTURE=1 NSBU_N512_TEMPORAL_REVIEWED_HOST=sulaco \
  python3 scripts/h32_launch_supervisor.py preflight-only --stage <r5-stage> \
  --plan-sha256 ac63c8c71e958d9a0b8e246338a1cccc09373eb9dde9084ea9012a87daa5acb9
                                                 # host_mismatch REFUSE off sulaco (baccus)
NSBU_RUN_N512_TEMPORAL_CAPTURE=1 NSBU_N512_TEMPORAL_REVIEWED_HOST=sulaco \
  python3 scripts/simulated_sulaco_preflight.py --stage <r5-stage> \
  --plan-sha256 ac63c8c71e958d9a0b8e246338a1cccc09373eb9dde9084ea9012a87daa5acb9
                                                 # contract-path simulation: ONLY the hostname gate simulated;
                                                 # every displayed reading is a REAL BUILD-HOST value, not Sulaco's
cd ../../avx-scheduled-endpoint/harness          # RUN_SOURCE = sha256 of docs/source-sha256.list
RUN_SOURCE=fa441601… cargo test --release --features n512-m512-temporal-h32   # 32 pass
RUN_SOURCE=fa441601… cargo test --release                                      # 18 pass
RUN_SOURCE=fa441601… cargo test --release --features n512-m512-piecewise-cadv33  # 28 pass
RUN_SOURCE=fa441601… cargo clippy --release --features n512-m512-temporal-h32 --all-targets -- -D warnings
NSBU_SNAPSHOT_FIXTURE_OUT=… NSBU_SNAPSHOT_LAYOUT_OUT=… NSBU_IDENTITY_DOC_OUT=… \
  RUN_SOURCE=fa441601… cargo test --release --features n512-m512-temporal-h32 fixture
                                                 # byte-identical to tests/fixtures + docs
python3 scripts/source_inventory.py              # deterministic 610-entry inventory
python3 scripts/leak_census.py --repeats 2       # 308 jobs, bad=0, live/Z/unconfirmed=0
python3 scripts/runner_kill_census.py            # 48 probes; sealed r5 run stranded=18,
                                                 # current-byte r6 runs stranded=17 (timing diagnostic, see
                                                 # docs/leak-census.stdout); exits 1 by design; all swept:
                                                 # per-probe unconfirmed=[], final external sweep 0
```

The selftest drives the real driver and the real reviewed watchdog with a fake
solver in `/tmp`; its first-step bundle is structurally exact (writer layout,
real digest) while the 95 simulated successor bundles are small non-sparse
placeholders, and nothing measured there is a scientific claim. The simulated
Sulaco probes are **contract-path simulations, never real Sulaco resource
readiness**: the guarded selftest runs on a shared build host and stubs the
hostname (`sulaco`), the host process-exclusivity census (empty observation
list), `MemAvailable` (250,000,000,000 B) and disk-free bytes
(400,000,000,000 B) around the fake solver, so its `PASS` exercises the
decision/cleanup contract only; `scripts/simulated_sulaco_preflight.py`
overrides only the hostname gate, so its `READY` numbers are this build
host's real readings, which certify nothing about Sulaco's memory, disk or
process exclusivity. Host process-exclusivity is proven separately (real
baccus refusal evidence + unit tests); actual Sulaco readiness and
exclusivity can only be established at launch time on sulaco itself through
the un-spoofable gates. `launch` mode
exists but was **not** run against any real solver: root reviews the plan and
deadline, then sulaco decides. Nothing here qualifies a PDE window:
`qualification=false`, `accepted_windows=0`.
