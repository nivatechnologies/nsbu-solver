# P10 offline captured-observer (experimental, diagnostic only)

Read-only conservative balance observer for **already decoded captured spectral
states**. It reuses the existing solver mathematical kernels and never
reconstructs, integrates or resets a `SpectralState`, and it provides **no
checkpoint/resume capability**. `scope=balance-diagnostic-only` and
`qualification=false` in every output: this is a doubled-grid scope balance
diagnostic, not full fine-observable acceptance, and it qualifies no PDE
window. No N512 run was performed; capability claims stay out of scope.

## Layout

- `harness/` — standalone crate `p10-offline-captured-observer`.
  - `src/observer.rs` — pure observer + checked conservative resource preflight ledger.
  - `src/main.rs` — CLI (`preflight` / `run`), profile admission, output binding.
  - `src/decode.rs`, `src/model.rs` — **included verbatim** via `#[path]` from
    the reviewed `evidence/p10/snapshot-comparison-adapter/harness/src/`; no
    copy, no weakened admission. Endpoint clocks (`remaining == 0`) and
    non-v2 cases and method-diagnostic profiles are explicitly refused on top of the decoder's
    admissions.
  - `src/control.rs` (test-only) — compiles the reviewed
    `evidence/p10/avx-w3-n256-integration-20260912/harness/src/observer.rs`
    `ReducedObserver` verbatim as the comparison control.
  - `src/fixtures.rs`, `src/tests.rs` (test-only) — small reviewed-format
    fixtures and the focused test suite.
  - `clock1536-record.json` — byte-identical copy of
    `work/live-input-metadata/clock1536-record.json`, used only as
    identity-length input for the arithmetic-only `n512-ledger` mode; the
    record's state itself was never loaded or allocated here.

## Numerical reuse

`OfflineObserver::observe` takes a validated `Domain` at construction, an exact
`TickClock`, and borrowed coefficient slices. Per sample it runs the same
kernels in the same order as the existing `ReducedObserver::sample`:
`ParallelReducedV2Force::begin_attempt` then `evaluate` on the doubled
diagnostic grid, `ConservativeWorkspace::evaluate`, `transfer` to the doubled
layout, and `balances::measure`. Inputs are validated (`validate_spectrum`) and
never mutated. `begin_attempt` is invoked on every observation exactly as the
reference does, so a reused observer keeps the reference attempt lifecycle and
stays correct if the force owner ever tightens its per-attempt boundary; with
the current provider the hook is a no-op and `evaluate` still enforces the
limits/termination contract.

## Resource preflight

`OfflineObserver::preflight` returns a checked admission ledger before any
allocation. The catalog, force-storage and conservative-workspace entries are
exact library reservations; the observer-array, resident-snapshot and header
entries are conservative allowances. The total is therefore a conservative
admission **upper bound**, not a measured allocator peak; the JSON output
labels the entries accordingly (`basis`, `exact_reservation_bytes`,
`conservative_allowance_bytes`).

`OfflineObserver::new` does not trust the ledger it is handed: it re-derives
every field from the actual construction inputs (source, samples, workers,
catalog backend, snapshot identity length) and refuses any ledger that differs
(`InvalidPayload`) **before allocating**, so a forged, shrunken or
cross-profile `total_bytes` cannot bypass the cap. Only a re-derived ledger
that fits the caller cap allocates; cap-minus-one is refused and tested.

## CLI

```sh
cd evidence/p10/offline-captured-observer/harness
cargo run --release --offline -- <preflight|run> <manifest.json> <force-sample-dim> \
    <workers> <cap-bytes> <owned-radix|rustfft-6.4.1-avx-avx2-fma>
cargo run --release --offline -- n512-ledger <clock-record.json> <cap-bytes>
```

JSON goes to stdout for external exclusive publication (no overwrite logic).
Output binds the manifest identity and all SHA-256 hashes, the exact clock
(exponent/target/elapsed/remaining), the full evolution block, the requested
sample dimension/workers/cap/backend, the classified ledger (exact
reservations vs conservative allowances, upper-bound `total_bytes`), and the
balance values (all verified finite). `preflight` adds `fits`; `run` additionally
reports the reverified snapshot hashes. Non-finite measurements, tampered
provenance, unsupported profiles and endpoint clocks exit non-zero.

`n512-ledger` is a **resource-only arithmetic mode**: it reads the identity
string length from the copied clock record and returns the same classified
conservative ledger for the live N512 profile (domain `[512;3]`, lengths
`[1,1,1]`, viscosity 1, force samples `[1024;3]`, 32 workers, AVX backend).
It performs no availability check and allocates no state, observer, catalog,
force table or snapshot, and it labels the Baccus 512 GiB comparison as an
estimate requiring a fresh actual-memory measurement before any N512 run.
The current conservative upper bound on this profile is 235,548,209,862 bytes
(~219.4 GiB), i.e. 314,207,604,026 bytes below the 512 GiB nominal estimate
(`fits_baccus_512_gib_nominal_estimate=true`); this is not a measured allocator
peak and does not by itself authorize a run.

## Verification performed (small only)

`cargo test --release --offline -j 2 -- --test-threads=1` independently passes
all 12 tests (26.53 seconds after compilation). The worker also passed Clippy
on the initial 11-test candidate; final combined Clippy is recorded separately.

1. `run` binds hashes/clock/parameters and stays `qualification=false`;
2. exact-cap run allocates, cap-minus-one refuses (`preflight` reports
   `fits=false`, `run` exits with the refusal);
3. tampered file/coefficient manifest hashes refuse;
4. method-diagnostic/HO profiles and zero-remaining endpoint clocks refuse;
5. on an **actual committed N8 CM state** (one loose-tolerance accepted
   transactional step), balances equal the existing `ReducedObserver`
   with exact floating-point equality, the input coefficient hash is unchanged after observation,
   wrong input shape is refused, and cap-minus-one allocation is refused;
6. forged ledgers (halved total, shrunken force storage, wrong backend,
   shortened snapshot bytes) and cross-profile ledgers (different samples
   layout or worker count) are refused with `InvalidPayload` even at
   `usize::MAX` cap — i.e. before any allocation — while the genuine ledger
   at the exact cap still constructs;
7. three repeated observations on one reused `OfflineObserver` each match a
   fresh `ReducedObserver::sample` (same `begin_attempt`→`evaluate` lifecycle),
   and the input state hash is unchanged;
8. AVX backend, minimum useful configuration: this host admits AVX2/FMA and
   the closed length set contains 128, so `preflight` admits source **N64**
   with doubled **N128** diagnostic and **M128** force samples. On an actual
   committed CM state (one loose-tolerance accepted transactional step from
   rest on an owned-radix RHS; the AVX N≤16 refusal below still stands), the
   AVX `OfflineObserver` at the exact cap matches the same-backend
   `ReducedObserver` with exact floating-point equality, refuses cap-minus-one with `ResourceLimit`,
   keeps the ledger inside the 2 GiB conservative budget, produces finite
   balances, and leaves the input state hash unchanged. An explicit
   `ensure_available` failure or N64 admission failure panics the test rather
   than skipping; the tiny N8/N16/M16 grid is separately asserted to stay
   refused (`InvalidDomain`), and the closed set [6, 96, 128, 144, 192, 256,
   288, 384, 512, 576, 768, 1024, 1152, 1536] means N≤16 grids remain
   unverified by design;
9. the v3 wireup clock (target 8192, endpoint elapsed 4096) yields
   remaining 4096 > 0 and is **permitted** by the exact-clock admission — the
   endpoint-4096 comparison is not a zero-remaining endpoint; only
   `remaining == 0` refuses;
10. `n512-ledger` on the copied record reports the classified arithmetic
    ledger for the live profile without any allocation, refuses cap-minus-one
    and accepts cap-exact `fits`, and carries the estimate-only nominal note.

The additional case-admission regression admits the frozen exact-v2 case and
refuses an otherwise valid manifest with one case-hash character changed.

Fixtures use only decoder-admitted profiles (N8, M384 CM `MATCHED_SPATIAL` /
`METHOD_DIAGNOSTIC` envelopes). The committed N64 state step runs slowly in the
unoptimized test profile (~10 min); no optimized N512 path was executed. Small
tests do not qualify any window; N512 and all PDE acceptance remain out of
scope. Real N512 captures exist on Sulaco, but **this candidate has NOT been
tested on them** — the N512 evidence here is arithmetic preflight only.
