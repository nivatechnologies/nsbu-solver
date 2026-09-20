# P10 offline N512 exact-v2 analytical reference observer (prepared, unqualified)

Read-only comparison of **already decoded immutable captured spectral states**
against the independent analytical exact-v2 reference at each snapshot's own
physical clock. This is a diagnostic preparation package: no N512 observation
was executed here, outputs are unqualified (`qualification=false`,
`accepted_windows=0`), and no PDE window is admitted. The observer never
evolves, resets, replaces, injects, resumes, recenters, aligns or phase-shifts
an integrated state; captured coefficients are borrowed, validated and
re-hashed unchanged after observation.

## Layout

- `harness/` — standalone crate `p10-n512-analytical-reference-observer`.
  - `src/main.rs` — CLI (`preflight` / `run` / `n512-ledger`), profile and
    identity admission, clock synchronization, result publication.
  - `src/decode.rs`, `src/model.rs` — **included verbatim** via `#[path]` from
    the reviewed `evidence/p10/snapshot-comparison-adapter/harness/src/`; the
    verified offline captured-state loader is used through
    `read_external_reference_manifest` (cox-matthews with M384/M512
    integration-force grids) without weakened admission. Endpoint clocks
    (`remaining == 0`), foreign v2 time identities, non-v2 cases, non-unit
    geometry and method-diagnostic profiles are refused on top of the decoder.
  - `src/ledger.rs` — pure-arithmetic admission ledger: exact library
    reservations (catalog, force storage, conservative workspace, both
    derivative samplers, reference caches) plus explicitly labelled
    conservative allowances (observer arrays, comparison arrays, resident
    snapshot, headers). The total is a conservative upper bound, not a measured
    allocator peak; `ReferenceObserver::new` re-derives every field from the
    actual inputs and refuses a forged, shrunken or cross-profile ledger
    before allocating.
  - `src/observer.rs` — allocation-gated workspace and the observation
    sequence: prescribed exact-v2 force evaluation and conservative mean-zero
    pressure construction on the doubled grid (the reviewed offline balance
    sequence), then sequential derivative sampling of the retained bands.
  - `src/quantity.rs` — velocity, complete gradient, ordered Hessian, curl,
    scalar pressure and pressure gradient compared pointwise to the cached
    independent analytical reference, with the reviewed component/derivative
    order, reviewed global collectors (`TensorErrors`) and reviewed global +
    declared regional masks (`RegionalTensorErrors`: core, annulus,
    interior-outside-nominal, collar, exterior) plus reviewed-contract peak
    height/location witnesses (first maximizer, x-major z-fast order).
  - `src/cache.rs` — one cached reference evaluation per velocity-lattice
    point (the reviewed evaluator output verbatim) and one compact four-column
    analytical pressure row per doubled-grid pressure sample.
  - `src/report.rs` — JSON evidence assembly and the all-finite gate.
  - `src/provenance.rs` — bounded streaming manifest/plan reads, strict JSON
    parsing with duplicate-key refusal, strict identity-field parsing and the
    full manifest-to-snapshot provenance bind (agreement between the raw and
    deserialized reads of the manifest, hex shapes, bounded identity,
    source/case/retained identity bindings, digest-bound plan file, embedded
    snapshot identity equality).
  - `src/plan.rs` — semantic validation of the digest-bound plan against the
    approved schema `p10-n512-analytical-reference-plan-v1`: every contract
    field is required and type-checked (a present-but-wrong-typed field is
    refused with its exact type error, never skipped), source commit
    (`source_commit` / `harness_commit_and_run_source`) bound to the snapshot
    source, `production_source_commit`/
    `numerical_test_source_commit` bound to the identity fields when present,
    profile binding, exactly the approved named artifact hashes
    (`binary_sha256`, `watchdog_sha256`, `source_sha256` — all required, all
    64-hex; any additional `*_sha256` field is an unapproved artifact hash and
    is refused), honest claims (`qualification`, `launch_authorized` and
    `run_authorized` must each be present and exactly `false`; an absent flag
    is an unbacked claim), retained layout equal to the snapshot grid,
    integration force layout equal to the snapshot evolution, required
    `endpoint_ticks` equal to both the identity endpoint field and the
    snapshot clock target, strictly ascending in-window observer nodes that
    each sit exactly on a scheduled landing tick of the plan schedule
    (including the segment-boundary landings) and contain the snapshot clock,
    and a complete contiguous schedule from tick 0 to exactly the endpoint in
    which every segment's span is an exact multiple of its own step and the
    schedule's leading segments equal the manifest evolution schedule
    segment-for-segment (the plan cannot silently re-define how the captured
    trajectory was integrated).
  - `src/record.rs` — strict semantic validation of the arithmetic ledger's
    clock record: the structured schema must agree with the identity `schema`
    field, the canonical 40-hex `source_commit` string is required and must
    equal the identity `source` field, identity keys are non-empty, the clock
    lies strictly inside the identity endpoint window and the `state_sha256`
    artifact hash is 64-hex; wrong types are refused, never skipped.
  - `src/n512.rs` — arithmetic-only `n512-ledger` mode with the fully bound
    identity echo (source, record source commit, profile, method, schemas,
    endpoint, clock, epoch, accepted steps, state hash, coefficient bytes)
    and the external source-binding block.
  - `src/source_bind.rs` — the exact `#[path]`-included decoder and model
    bytes are embedded into the binary at compile time with `include_bytes!`
    (as is `../source-inventory.json`), so every published output carries the
    SHA-256 of the bytes it was actually compiled from — never a runtime
    re-read — plus the sealed inventory's own digest, both path-dependency
    crates' aggregate digests recomputed from the compiled-in inventory and
    cross-checked against the sealed aggregates, and the full per-file
    inventories. `src/surface_tests.rs` proves the binding arms refuse
    foreign inventory shapes.
  - `src/publication.rs` — create-only result publication: an existing output
    path is refused; results are written exactly once via a synchronized
    same-directory temporary attached with `hard_link`; cleanup, completeness
    and parent-directory synchronization failures are terminal and report the
    kernel's genuine outcome (an ENOENT cleanup failure states the temporary
    is absent, never that a leftover remains; completeness rollback removes the
    attachment and synchronizes the parent; the parent synchronization after a
    successful attach runs even when the temporary's cleanup is denied, so a
    published result is never reported without the durability sync that makes
    it durable). Residue statements are prove-before-claim: the refusal may
    only say a file remains after an existence check actually succeeds, and a
    check that itself fails states the uncertainty instead. A pre-existing
    stale temporary at the derived `.name.<pid>.tmp` name is refused before
    anything is written. The injection faults drive the production code paths
    at the real kernel boundaries — the directory-sync fault revokes the
    parent's permissions so the real `open`/`sync` fails with
    `PermissionDenied`; the denied-cleanup faults revoke only the unlink or
    only the re-open — and each refusal states the uncertainty it leaves
    behind.
  - `../source-inventory.json` — sealed source bindings: the decoder/model
    digests (matching the compile-time-embedded `external_source_bindings`)
    plus the complete per-file SHA-256 inventories of both path-dependency
    crates (`nsbu-benchmarks`, `nsbu-solver`); `src/inventory_tests.rs`
    re-hashes every pinned file, checks the inventories are complete against
    the files on disk, verifies the compiled bytes and both aggregate digests
    and the sealed inventory digest appear in published outputs, checks the
    outputs' digests equal what is on disk right now, and carries the
    rebase regression: an inventory still sealing the stale a7f7-era decoder
    digest is refused against the compiled bytes while the sealed current
    30dc45f decoder digest binds.
  - `src/fixtures.rs`, `src/tests.rs`, `src/strict_tests.rs`,
    `src/binding_tests.rs`, `src/known_value_tests.rs`,
    `src/coverage_tests.rs`, `src/arm_tests.rs`, `src/guard_tests.rs`,
    `src/schedule_tests.rs`, `src/mutation_tests.rs`,
    `src/publication_fault_tests.rs`, `src/surface_tests.rs`,
    `src/inventory_tests.rs`, `tests/cli_adversarial.rs` —
    reviewed-format tiny fixtures, the focused positive/refusal suite,
    tampering refusals, source/plan/record binding refusals, plan
    flag/artifact-hash/schedule/observer-node adversarial refusals,
    real-CLI mutation refusals (wrong-typed rehashed plan fields through
    consistently re-digested manifests, wrong-typed or missing canonical
    record source commits, forged capture claims, work-cap overrun,
     truthful publication-fault reporting), the full included-decoder surface
     (M384 comparison admission, the reviewed Cox--Matthews time-diagnostic
     M384/M512 profile arms with a foreign M448 refusal,
     envelope/evolution/schedule arms, arithmetic-control binding on both the
     M384 and M512 force grids with a cross-grid refusal, every
     snapshot-reader refusal), real subprocess CLI adversarial regressions,
     source-inventory verification,
    independent closed-form known values (derivatives, Hessian,
    curl, gauge, peak witnesses), and the fixed 50-transform / 15 GiB
    correction guards. No test allocates anything close to N512.

## Numerical reuse (and its boundary)

Reused unchanged through public APIs: the independent reference evaluator
`nsbu_benchmarks::fields::reference::evaluate`, the prescribed-force provider
`ParallelReducedV2Force`, `ConservativeWorkspace` (mean-zero pressure),
`DerivativeWorkspace` sampling, `TensorErrors`/`LocalError` global statistics,
and `RegionalTensorErrors`/`classify` geometric masks. The family-specific
in-situ trackers (`v2_experiment::reference`, `pressure_reference`) are wired
to live six-branch `SpectralState` families and are `pub(crate)` internally,
so this adapter reproduces only their public pointwise wiring and call order
on borrowed captured slices; that mirroring is deliberate and reviewed code is
never edited. Peak witnesses mirror the reviewed `physical/extrema` contract
because `PhysicalExtrema::from_comparison` is `pub(crate)`; witnesses are
constructed with the reviewed first-tie/finite/nonnegative rules.

## Pressure gauge handling

The actually constructed pressure is mean-zero by the reviewed conservative
projection. The analytical scalar pressure is compared after subtracting the
**explicitly measured analytical sample-lattice mean** (deterministic ordered
sum over the declared pressure lattice, reported in full). Pressure-gradient
comparison is gauge-independent. The imported high-precision empirical gauge
artifact remains a separate required comparison; every output records
`imported_high_precision_gauge_artifact: not-imported;separate-required-comparison`.
Local/regional pressure means are never removed independently.

## Identity and clock binding

The adapter binds: exact case SHA-256 (must equal both the manifest
`evolution.case_sha256` and the snapshot identity `case=` field and the frozen
`similarity-mms-v2` `CASE_SHA256`), snapshot identity string, `retained=` grid
field versus manifest dimensions, non-empty `provider=` field, source commit,
frozen-plan path/hash, snapshot file and coefficient SHA-256 (loader-verified),
optional profile binding, and one exact `TickClock` restored from the manifest
clock (exponent, target, elapsed, positive remaining) that simultaneously
drives the state sampling, the force, the reference and every classification.
The loader already proves the snapshot header equals the manifest clock. The
published clock block carries the separate `BenchmarkTime` elapsed/remaining
dyadic-conversion rounding estimates (`physical_rounding_estimates`; exact
significands receive zero). The arithmetic-only `n512-ledger` mode binds the
clock-record identity the same way: `case=` must equal the frozen
`CASE_SHA256`, `retained=` must equal the 512 ledger profile and `provider=`
must be non-empty, alongside `resumable=false` and `qualification=false`; the
record must additionally carry a structured schema agreeing with the identity
`schema=` field, a 40-hex identity `source`, the record's own required canonical
40-hex `source_commit` string equal to that identity `source` (a missing or
wrong-typed value is refused, never skipped), the cox-matthews method, hex-bound
`production_source`/`test_source` fields when present, a clock strictly inside
the identity endpoint window, positive epoch/accepted-step counts and a
64-hex `state_sha256` artifact hash, and the output echoes every bound field.
Observer runs additionally parse the digest-bound plan text and refuse plans
whose source, profile, layouts, endpoint, schedule or clock semantics disagree
with the snapshot (see `src/plan.rs`), and manifests are read through bounded
streaming with duplicate-key and path-confinement refusals (see
`src/provenance.rs`).

## Work and memory caps

`preflight` and `run` compute the exact per-attempt work ledger before any
allocation: cached reference evaluations (velocity points + pressure points),
the reviewed 128-iteration root allowance per evaluation, the declared
root-budget classification allowance for all six quantity collectors, the
observer/conservative/provider transform inventories and weighted visits.
Reference evaluations above `max-reference-evaluations` and total ledger bytes
above `cap_bytes` are refused before allocation. `ReferenceObserver::new`
re-derives the ledger and refuses cap-minus-one and forged ledgers.

## N512 preflight (arithmetic only)

```sh
cd evidence/p10/n512-analytical-reference-observer/harness
cargo run --release --offline -- n512-ledger \
    ../../offline-captured-observer/harness/clock1536-record.json \
    512 1024 1024 32 1099511627776 600000000000
```

With the velocity lattice at the retained 512³ and the pressure/force lattice
at the doubled 1024³, the conservative upper bound is 366,546,289,318 bytes
(headroom 183,209,524,570 versus the 512 GiB nominal after the live-array term
was corrected upward by 16,106,127,360 bytes), and the work ledger charges
1,207,959,552 cached reference evaluations. This mode is arithmetic
only: no availability check and no allocation of state, observer, catalog,
force table, caches or snapshot. A fresh actual-memory measurement and a
bounded reviewer decision are still required before any N512 observation; the
1.2e9-evaluation reference cost is why reference-precision refinement and grid
selection remain separate decisions.

## CLI

```sh
cargo run --release --offline -- <preflight|run> <manifest.json> \
    <velocity-sample-dim> <pressure-sample-dim> <force-sample-dim> <workers> \
    <root-budget> <max-reference-evaluations> <cap-bytes> \
    <velocity-floor> <pressure-floor> <owned-radix|rustfft-6.4.1-avx-avx2-fma> \
    [output.json]
cargo run --release --offline -- n512-ledger <clock-record.json> \
    <velocity-sample-dim> <pressure-sample-dim> <force-sample-dim> \
    <root-budget> <max-reference-evaluations> <cap-bytes> [output.json]
```

`[output.json]` engages create-only publication; without it JSON goes to
stdout. Every output labels `scope=analytical-reference-observer:unqualified-sampled-diagnostic`,
`qualification=false`, `accepted_windows=0` and
`reference_precision_refinement=required-separate-comparison:not-performed-here`.
This is not a trajectory, resume, state-injection, tolerance, convergence or
PDE-qualification interface.

## Verification

`cargo test --offline` (91 in-process tests plus 4 real-subprocess CLI
adversarial regressions: positive binding with zero clock rounding estimates,
global/regional/peak/gauge completeness, preflight, create-only publication,
tampered hash/case, unsupported profiles/method/domain, endpoint and foreign
v2 clock refusals, work and cap-minus-one refusals, forged-ledger refusal,
read-only re-hash, bounded CLI-surface refusals, strict-parser and
path-confinement refusals, manifest-read disagreement refusals, plan/record
semantic binding refusals, plan flag/named-artifact-hash/schedule/
observer-node adversarial refusals, the full included-decoder surface
(M384 comparison admission, the reviewed Cox--Matthews time-diagnostic
M384/M512 profile arms with a foreign M448 refusal, envelope/evolution/
schedule arms, arithmetic-control binding on both the M384 and M512 force
grids with a cross-grid refusal, every snapshot-reader refusal), the
rebase source-binding regression (a stale a7f7-era decoder inventory
refuses the compiled bytes; the current 30dc45f decoder inventory binds),
real-CLI
mutation refusals for wrong-typed rehashed plan fields, wrong-typed or
missing canonical source commits, forged capture claims and work-cap
overrun, truthful publication-fault reporting including stale-temporary
refusal and prove-before-claim residue statements, sealed source-inventory
and compile-time binding verification, independent closed-form known
values, and the fixed 50-transform and 15 GiB-correction guards including
the N512 arithmetic-ledger paths) and
`cargo clippy --offline --all-targets` with `clippy::all` denied.
`cargo +nightly-2026-03-03 llvm-cov --offline --all-targets --branch`
exported across every test, integration-test and binary object reports
**97.99% line, 81.54% branch, 96.09% function and 97.01% region** coverage
inclusively — including the verbatim included decoder/model and all
test/fixture modules, with no file excluded. `rust-code-analysis-cli`
reports every function (closures included) at cyclomatic ≤20 and cognitive
≤10 with Halstead difficulty ≤30, and the repository
`quality/check_crap.py` gate passes with per-function CRAP computed from
the LLVM branch data at a maximum of 22.5 (<25). The raw measurement
artifacts (`coverage.json`, `rca.jsonl`, `crap.json`, commands and the
gate summary) are recorded under `quality/`. All sources are below 500
lines and fixtures stay at N=8 grids.
