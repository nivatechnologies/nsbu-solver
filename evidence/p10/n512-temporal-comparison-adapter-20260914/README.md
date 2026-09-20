# N512 temporal comparison adapter (preparation-only)

This packet qualifies the read-only preparation adapter for the P10 N512/M512
temporal comparison at the eight shared clocks 512, 1024, 1536, 2048, 2560,
3072, 3584 and 4096. The reusable tooling and tests live in the repository:
`tools/prepare_temporal_comparison.py`, `tools/temporal_comparison_contract.py`
and `tools/tests/{temporal_comparison_fixtures,test_prepare_temporal_comparison,
test_prepare_temporal_comparison_refusals,test_temporal_comparison_contract,
test_prepare_temporal_m512_evidence}.py`,
documented in
[`P10_TEMPORAL_COMPARISON_ADAPTER.md`](../../../docs/P10_TEMPORAL_COMPARISON_ADAPTER.md).
It compares independently evolved h64/h32/h16 N512/M512 Cox--Matthews
trajectories by manifest bookkeeping only; it never opens a state payload,
never injects, resets or reference-substitutes an integrated state, and cannot
accept a PDE window, apply a tolerance or change one.

`family-plan.json` is the emitted closed family contract: the reviewed h64 v3
anchor (identity, profile, source and plan hash bound exactly; 48 attempts;
32 steps of 64 through clock 2048 and 16 steps of 128 through 4096) beside the
nested h32 (96 attempts) and h16 (192 attempts) branches, whose capture
identity, profile, source, plan and per-pair arithmetic reviews are explicit
`PENDING_*` bindings until real captures are reviewed. Strict admission refuses
any pending binding. The bound state payload is 3,233,808,384 bytes; the
plan's disk arithmetic binds 24 payloads totalling 77,611,401,216 bytes and
per-pair decoder reservations of 6,468,665,344 bytes.

The adversarial-review repair pass (2026-09-14) closed two defects without
changing the emitted packet bytes: branch schedules are now type-strict (float
or boolean tick values that merely compared equal to the closed integer ticks
are refused) and the adapter's manifest/bookkeeping literals are bound to the
contract constants instead of duplicated values.

Decoder-admission pass (2026-09-14): `decode::read_manifest` now admits M512
evolution (cox-matthews with `[512; 3]` integration-force dimensions) beside
the preserved M384 path, binds arithmetic-review force dimensions to the
manifest evolution for the exact M384/M512 values, and refuses any other M at
that earliest gate; the decoder admits evolution only and does not itself bind
the retained lattice — the exact N512 lattice for M512 time-diagnostic sides is
enforced by `compare::admitted_time_force_dimensions` at the comparison gate
in `compare.rs`. The 1547-line adapter-harness
`src/tests.rs` split into `command`, `time_diagnostics`, `fixed_diagnostics`
and `hessian_mixed` modules beside the existing modules, each below 500 lines,
and `tests/temporal_e2e.rs` replays the byte-exact producer publication through
`decode::read_manifest` beside the reviewed v3 launch plan (h64 side) and
co-located `arithmetic-evidence.json`, never opening the bound (absent) state
payload; `tests/m512_temporal_admission.rs` admits all four fixture sides
through `decode::read_manifest` with valid synthetic capture plans hash-bound
by resealed `plan_sha256` digests and retargets the h64--h32 sides through
post-2048 nested schedules; and `tests/coverage_refusals.rs` drives every
decoder, comparator and CLI refusal branch to the measured coverage gate below.
The shared `m512-temporal/` fixture manifests carry synthetic h32/h16
identities, plans and epoch records (the h64 side alone binds the actual
reviewed v3 identity, plan hash and plan bytes), so these fixtures demonstrate
decoder admission only — no real h32/h16 capture, comparison or window result
is claimed anywhere in them.

Astra-v4 repair pass (2026-09-19): closed the two review blockers without
touching any capture, plan or fixture byte. The strict typing gate now runs
through the explicit `.venv` environment named by the reviewed
`pyproject.toml` `[tool.basedpyright]` config, so the invocation exits 0 with
zero errors and records its exit status; and the documentation now states what
the gates actually do — `decode::read_manifest` admits M512 evolution while the
exact N512 lattice is enforced by `compare` — with the synthetic-epoch fixtures
explicitly scoped to decoder admission only.

Astra-v5 repair pass (2026-09-20): closed the independently reviewed blockers.
`validate_family_plan` (CC 51, cognitive 49) was split into seven contiguous
validators (`_validate_family_bindings`, `_validate_family_clocks`,
`_validate_family_physics`, `_validate_family_disk`,
`_validate_family_branches`, `_validate_family_pairs` and
`_validate_family_arithmetic_files`) with byte-identical check order, messages
and behaviour; the CC-23 bookkeeping test split into three tests that keep
every original assertion; and the `m512_temporal.rs` header comment now scopes
the synthetic fixtures to metadata decoding and manifest admission only, with
no numerical comparator trajectory claimed. The documented family-plan CLI was
corrected to `--emit-family-plan --output PATH` (the flag-then-path form had
never matched the parser) and that exact invocation was exercised, emitting
bytes identical to `family-plan.json`. The three `work-*.tmp` duplicates in the
harness (byte-identical copies of the archived coverage and test captures) were
removed. Every `smalltests` file was regenerated from the current tree: 185
focused Python tests, the seven-file line-and-branch coverage report, the
`rust-code-analysis-cli` per-function complexity report, the measured-scope
CRAP reports, the repository check, 50 harness tests, the pinned LLVM
branch-coverage summary and the typing run all pass as recorded below; exact
input hashes were added in `source-hashes.txt`; and `SHA256SUMS` was resealed
over the refreshed packet bytes.

Astra-v6 repair pass (2026-09-20): closed the two remaining integration
blockers. `src/m512_spatial.rs::is_r6_evolution` (CC 11, measured CRAP 26.125
in the v5 harness-wide disclosure) was split into four contiguous pure contract
checks — `is_r6_case`, `is_r6_flow`, `is_r6_lattice` and `is_r6_tolerances` —
keeping every conjunct, its order and the refusal message byte-identical, so
the behaviour is unchanged; the entry point now measures CRAP 4.0 at full
branch coverage. The new `src/tests/m512_spatial_evolution.rs` module keeps the
contract meaningfully tested: the closed evolution is admitted, each of
fourteen single-field deviations is refused (including a NaN viscosity that
pins the exact `to_bits` equality), and pair-equal mutations still refuse the
manifest pair through `is_r6_evolution` itself (53 harness tests). Rust LLVM
coverage was regenerated from a cleaned profile with the repository policy
flags `--no-default-ignore-filename-regex --ignore-filename-regex
'(/rustc/|/\.cargo/registry/|/\.rustup/toolchains/|/target/)'`, so the
`cfg(test)` modules are instrumented and the only exclusions are
registry/rustc/rustup/toolchain/target; per-function CRAP is now measured over
every instrumented source and test function (330 functions, maximum 23.42,
gate <25). The untracked v5 test modules were `cargo fmt`-normalized and made
`cargo clippy --all-targets -- -D warnings`-clean (function-pointer type
aliases; one needless borrow), keeping every file below 500 lines. The
inventory in `source-hashes.txt` now hashes all 21 measured harness `.rs`
files, the 7 measured Python files, the design doc and all 4 fixture manifests
(33 files) with exact wording — the v5 wording undercounted the measured Rust
set (v5 claimed 21 files while instrumenting nine and measuring complexity of
twenty).

Astra-v7 repair pass (2026-09-20): closed the independent temporal-v7 blocker.
The file-level Halstead difficulty of `src/mixed_math.rs` measured 87.94117647058823
under `rust-code-analysis-cli` 0.0.25, violating the <80 gate that the CI jq
expression applies to every unit and function. The cohesive arithmetic helpers
were split into two new modules — `src/mixed_math/kernels.rs` (the band
accumulators `MixedSums`, `BandSums`, `BandValues`, `NormSums` and `CrossSums`
with the output-composition helpers `split`, `add_norms`, `add_cross`,
`cross_output` and `cosine_similarity`) and `src/mixed_math/kernels/primitives.rs`
(the arithmetic primitives `curl`, `real_inner`, `Compensated`, `ScaledSquares`
and `finite`) — each conjunct, guard, floating-point operation order and refusal
message relocated byte-for-byte. `src/mixed_math.rs` keeps the published metric
types, the closed domain/value validation and the `calculate` mode-loop entry
point, so the public contract used by `mixed.rs` is unchanged and no test,
fixture or message was edited. Post-split file difficulties are 43.9322,
62.3103 and 48.8281, the harness-wide maxima are 77.9806 (file, `compare.rs`)
and 39.0 (function), and the exact CI jq gate expressions pass over the
harness-wide metrics ndjson (`smalltests/halstead.txt`). Behaviour equivalence
is recorded by the unchanged totals: 53 harness tests pass, the per-function
CRAP report holds the same 330 functions with identical scores (maximum 23.42,
only their module-path attribution moved), the changed-sources CRAP report is
byte-identical, and the LLVM totals are unchanged at 94.40% inclusive lines and
84.36% branches. Every affected evidence file was regenerated from the current
tree and the packet resealed.

## Evidence

- `smalltests/pytest-temporal-adapter.stdout` — 185 focused tests pass (exit 0;
  the 183 of the previous pass plus the two added by the bookkeeping-test
  split), including the four tests in
  `test_prepare_temporal_m512_evidence.py` (evidence materialization, exact
  nested schedules/epochs and two fail-closed refusal replays):
  happy-path publication of 16 pair-clocks x 2 manifest inputs plus the
  clock-ascending ledger, exact bundle/epoch/name derivation for all 24
  captures, payload and disk arithmetic, contract-constant-bound manifest
  evolution, determinism, capture-root invariance, unreadable payloads still
  completing, standalone-script CLI emission byte-identical to this packet
  (exercised through the documented `--emit-family-plan --output PATH` form),
  and the refusal matrices for missing, duplicated, renamed, relabelled,
  pending, tampered and identity-substituted metadata (nothing published on
  refusal), non-integer schedule ticks, oversized streamed inputs at their
  byte bounds, path-escaping bindings, lineage/arithmetic conflicts and the
  publication-uncertainty exit 2 that retains both the raced destination and
  the staged copy while leaving the evidence set incomplete (fail-closed).
  The h64 side carries the ACTUAL reviewed v3 identity, plan hash and plan
  bytes.
- `smalltests/coverage.txt` — coverage of the seven measured scope files (the
  two adapter modules, the shared fixture module and the four focused test
  files) from the single 185-test run above, recorded with its exact commands.
  Measured separately: executable lines 1229/1231 = 99.84%; branches
  278/280 = 99.29%; both exceed the >=80% gate. The two unexecuted lines are
  defensive refusals unreachable against the closed family constants. No
   other test is claimed as measured here; the `cfg(test)` Rust modules are
   measured by the policy-flagged LLVM coverage run recorded in
   `rust-coverage.txt`, not by this report.
- `smalltests/pytest-tools-suite.stdout` — the whole `tools/tests` suite:
  405 passed, 3 skipped and 9 failures, all nine reproduced identically on the
  pristine base commit `a7f7a57` in this same environment (missing
  `mpmath`/venv dependencies for unrelated verification-runner tests); every
  temporal-adapter test passes.
- `smalltests/repository-check.json` — `tools/check_repository.py` passes with
  zero errors after this packet's links exist (exit 0).
- `smalltests/file-sizes.txt` — all 32 measured files (23 harness `.rs`, 7
  Python, the design doc and this README) are below 500 lines; the maximum is
  499 (`src/tests/coverage_refusals.rs`).
- `smalltests/basedpyright.txt` — basedpyright 1.40.1 strict run over the two
  adapter modules and all five temporal test files (including
`test_prepare_temporal_m512_evidence.py`): 0 errors, exit status 0, recorded
  in the file with the exact invocation, cwd, environment and config. The
  invocation uses the explicit `.venv` environment that the reviewed
  `pyproject.toml` `[tool.basedpyright]` config names (`venvPath=.`,
  `venv=.venv`), so no venv-resolution warning remains. (The original packet
  recorded no typecheck because the host lacked basedpyright; an earlier
  re-check used a host install whose configured `.venv` was absent, printing a
  venv warning and exiting 3; this pass installs basedpyright 1.40.1 plus
  pytest 9.0.3 into that `.venv` and re-runs to exit 0 with zero errors.)
- `smalltests/rust-tests.stdout` — the adapter harness: 53 tests pass (exit 0), including
  the four synthetic-epoch byte-exact fixture sides admitted through
  `decode::read_manifest` with hash-bound synthetic plans and co-located
  evidence (metadata/decoder admission only — the `m512_temporal` header now
  states plainly that those fixtures exercise manifest metadata and refusal
  branches, not a numerical comparator trajectory), both pairs' sides
  through post-2048 nested schedules, and the refusal matrix for tampered,
  diverged, unparseable and unsealed arithmetic evidence, malformed schedules,
  corrupt snapshots, broken envelope/digest/tolerance/guard bindings,
  clock/lineage/schedule divergence and CLI usage/cap malformations.
- `smalltests/rust-coverage.txt` — `cargo +nightly-2026-03-03 llvm-cov --branch
  --no-default-ignore-filename-regex --ignore-filename-regex
  '(/rustc/|/\.cargo/registry/|/\.rustup/toolchains/|/target/)'` summary over
  the harness from a cleaned `target/llvm-cov-target` profile (the pinned
   coverage toolchain): all 23 harness `.rs` files appear, including the
  `cfg(test)` modules, because only registry/rustc/rustup/target paths are
  excluded. Actual LLVM branch coverage (branch counters, no region
  substitution) is 94.93% for `decode.rs`, 96.67% for `compare.rs` and 91.38%
  for `m512_spatial.rs` — the changed sources — and 84.36% harness-wide with
  94.40% inclusive executable lines; every figure and the inclusive line and
  branch totals satisfy the >=80% gate (region percentages stay in the same
  file and do not enter any gate). The v5 run used `cargo-llvm-cov`'s default
  filename-ignore policy, whose report listed only the nine `src/*.rs`
  production files and omitted every `cfg(test)` module from measurement.
- `smalltests/complexity.txt` — per-function cyclomatic and cognitive
  complexity, measured over the seven temporal Python sources/tests
  (`radon` 6.0.1 cyclomatic + the SonarSource cognitive-complexity algorithm)
  and all 23 harness `.rs` files including the `cfg(test)` modules
  (`rust-code-analysis-cli` 0.0.25): 497 functions, maximum cyclomatic 21 and
  maximum cognitive 21 — every source and test function below both gates,
  after splitting the former CC-51 `validate_family_plan`, the CC-23
  bookkeeping test and the CC-11 `is_r6_evolution`. The v5 report claimed 21
  files while the tree held twenty; the v6 tree genuinely held twenty-one and
  the v7 tree holds twenty-three (the mixed_math split adds `kernels.rs` and
  `kernels/primitives.rs`); the report's scope line states the toolchain
  exactly.
- `smalltests/halstead.txt` — the v7-file-level Halstead record: the exact CI
  jq gate expressions (`.github/workflows/rust.yml`) run verbatim over the
  regenerated 23-file harness metrics ndjson, each exiting 0 (`true`), with
  every unit's file-level difficulty listed. The blocker value,
  `src/mixed_math.rs` at 87.94117647058823, is closed by the v7 split to
  43.9322 / 62.3103 / 48.8281; the harness-wide maxima are 77.9806 (file,
  `compare.rs`) and 39.0 (function), both below 80.
- `smalltests/python-crap.json` — measured per-function CRAP over the seven
  measured scope files, from `quality/check_crap.py python` against the
  coverage JSON of the same 185-test run and the Radon CC report: 167
  functions, maximum 18.0, gate <25. Radon 6.0.1 truncates a function's
  endline when its last statement is a bare `assert` (`visit_Assert` skips
  line tracking), which would falsely score fully covered test functions as
  uncovered; the CC report's `endline` fields were therefore replaced with the
  exact `ast` end line numbers of each named definition before scoring, as
  recorded in this packet's reproduction section.
- `smalltests/rust-crap-changed-sources.json` — measured per-function CRAP for
  the three changed harness sources (`compare.rs`, `decode.rs`,
  `m512_spatial.rs`) from `quality/check_crap.py rust` against the LLVM export
  JSON of the policy-flagged branch-coverage run: 97 functions, maximum 20.0,
  gate <25; the v7 regeneration of this report is byte-identical to the v6
  bytes because none of the three changed sources moved.
  `smalltests/rust-crap-harness-wide.json` is the same run over every
  LLVM-instrumented source and test function in all 23 files, `cfg(test)`
  modules included: 330 functions, maximum 23.42
  (`src/mixed.rs::validate_manifests`), gate <25. The v5 maximum, 26.125 at
  `src/m512_spatial.rs::is_r6_evolution`, is closed: the split functions score
  4.0/4.0/3.0/2.0 at full branch coverage. The v7 report holds the same 330
  functions with identical scores; only the relocated mixed-math functions'
  `path` fields changed.
- `smalltests/source-hashes.txt` — SHA-256 of every one of the 35 measured
  input files: the 23 harness `.rs` files (12 production and `cfg(test)` root
  modules — the 10 `src/*.rs` files plus the v7 `src/mixed_math/kernels.rs`
  and `src/mixed_math/kernels/primitives.rs` — and 11 `src/tests/*.rs`
  `cfg(test)` modules), the 7 measured Python sources/tests, the external
  design doc `docs/P10_TEMPORAL_COMPARISON_ADAPTER.md` and all four
  `m512-temporal/` fixture manifests.

## Reproduction (2026-09-20 pass)

```sh
cargo fmt --manifest-path evidence/p10/snapshot-comparison-adapter/harness/Cargo.toml -- --check
cargo clippy --manifest-path evidence/p10/snapshot-comparison-adapter/harness/Cargo.toml \
  --all-targets -- -D warnings                            # clean
.venv/bin/python -m pytest tools/tests/test_temporal_comparison_contract.py \
  tools/tests/test_prepare_temporal_comparison.py \
  tools/tests/test_prepare_temporal_comparison_refusals.py \
  tools/tests/test_prepare_temporal_m512_evidence.py      # 185 passed
python3 -m coverage run --branch -m pytest <the same four files>   # host python 3.12.3
python3 -m coverage json -o work/temporal-coverage-v7.json
python3 -m coverage report --include=<the seven measured files>
radon cc -s -j <the seven measured files>                 # Radon 6.0.1
# replace each report endline with the ast end_lineno of the same-named,
# same-lineno definition (Radon truncates functions ending in a bare assert)
python3 quality/check_crap.py python <span-fixed-cc.json> work/temporal-coverage-v7.json \
  smalltests/python-crap.json                             # 167 functions, max 18.0
rust-code-analysis-cli -p <file> -m -O json -o <dir>      # 0.0.25, all 23 harness files,
#   run inside the harness dir with -p src/... so unit names are repo-relative
#   (src/... and src/tests/...); concatenate the per-file JSON into one ndjson
jq -se '[.. | objects | select(has("metrics")) | .metrics.halstead.difficulty | select(. != null)] | max < 80' <ndjson>   # smalltests/halstead.txt
cargo test                                                # 53 passed
rm -rf target/llvm-cov-target                             # clean profile, from harness dir
cargo +nightly-2026-03-03 llvm-cov --branch \
  --no-default-ignore-filename-regex \
  --ignore-filename-regex '(/rustc/|/\.cargo/registry/|/\.rustup/toolchains/|/target/)' \
  --json --output-path work/rust-coverage-export-v7.json  # runs the 53 tests
cargo +nightly-2026-03-03 llvm-cov --branch \
  --no-default-ignore-filename-regex \
  --ignore-filename-regex '(/rustc/|/\.cargo/registry/|/\.rustup/toolchains/|/target/)' \
  --no-run --summary-only                                 # smalltests/rust-coverage.txt
python3 quality/check_crap.py rust <rca-ndjson> <llvm-export.json> \
  smalltests/rust-crap-harness-wide.json                  # 330 functions, max 23.42
python3 quality/check_crap.py rust <changed-sources-ndjson> <llvm-export.json> \
  smalltests/rust-crap-changed-sources.json               # 97 functions, max 20.0
.venv/bin/basedpyright <the seven measured files>         # 0 errors, exit 0
python3 tools/prepare_temporal_comparison.py --emit-family-plan \
  --output /review/n512-temporal-family-plan.json         # == packet family-plan.json
python tools/check_repository.py                          # status passed
```

## Limits and next gates

No h32/h16 N512 capture, whole-file inventory or serial/W3 arithmetic review
exists yet; none of the 24 states was compared here and no comparison was
executed. Remaining gates before any N512 temporal result: capture the h32 and
h16 branches from rest under separately reviewed plans; sync and review the
whole-file inventories; freeze the per-pair arithmetic lineage evidence; run
the adapter against the real root; then invoke the existing read-only
comparison adapter binary per pair at each clock. Passing this packet prepares
inputs only. Zero concentrating PDE windows are accepted.
