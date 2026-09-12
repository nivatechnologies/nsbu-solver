# Exact-v2 attempt-local force-cache evidence

This source-bound artifact records phase A at source commit `9836cfc9f3b13a6f3df39865bd016d17e6bc1e85`, based on integration commit `a5bf7644f8542f5a8124bf3c117de4e9dfe0ba5e`.

The core prescribed-force contract now has a backward-compatible, default no-op attempt hook. `SpectralRhs` calls it once after time/work admission and before enabling any RHS call. The opt-in `AttemptForceCache` owns only the deterministic original exact-v2 `RunForce`, invalidates all state on every attempted interval, and retains five coefficient slots for the exact clocks returned by `TickClock::stages`. It is absent from the default `v2_run::Run` and archive-v1 path.

On actual accepted N4 macro attempts, cached and uncached CM and HO runs produced bitwise-identical committed state coefficients. CM made 12 RHS calls and HO made 15; both made exactly five original-provider evaluations. Aggregate RHS work equals separately reported original-provider work plus exact-clock comparisons and coefficient copies. Aggregate transforms equal original-provider transforms plus ten spatial-operator transforms per RHS call.

The small N4 serial profile reserves 9,928 bytes for the original provider and 23,576 bytes for the cached provider. The latter includes all five three-component coefficient slots and allocator allowance. Preflight proves every direct-call counter through the conservative 15-call limit. Cap-shortfall and arithmetic-overflow controls refuse; the dedicated allocator harness observes no steady allocations.

Four integration tests, one bounds unit test, the allocator harness, six solver force-budget regressions and four archive-v1 regressions pass. Controls cover every admitted stage clock, repeated hits against fresh independent provider evaluations, calls outside an attempt, an off-manifest clock, wrong layouts/limits, the 16th call, a rejected macro attempt, and repeated underlying-provider failures. Failed fills never publish and a failed begin leaves the prior generation inaccessible.

Focused changed-production coverage is 416/428 lines (97.1963%) and 47/52 branches (90.3846%). Production CRAP peaks at 19.125. LLVM does not report integration-test functions; their conservative zero-coverage CRAP peaks at 12 because changed test functions have CC at most 3. Whole-crates static maxima are CC 21, cognitive 21, function Halstead difficulty 60 and all-node difficulty 75.929515. The largest tracked Rust file is 477 lines; the largest changed file is 337. Exact workspace formatting, all-target Clippy and strict Rustdoc pass.

This phase makes no runtime timing or scientific qualification claim. It does not add a cached run owner, change force sampling/arithmetic, cache observer work, persist values across attempts, or define a cached checkpoint. Default version-1 runs and archives remain uncached and their focused round trips pass.
