# N512/M768 ownership planning ledger

This is a formula review only. It starts no solver, allocates no numerical grid, creates no W3 admission, and is not a host cap, execution plan, or qualification result.

The current endpoint owner constructs the integration RHS and the observer together. Applying the existing reservation formulas to N512 retained state, M768 integration force, padded-768 RHS, and a doubled N1024/M1024 observer gives 238,210,972,824 bytes for integration and 232,283,722,008 bytes for the observer. Their simultaneous lifetime is 470,494,694,832 bytes. The integration and observer are therefore the source of the apparent greater-than-256-GiB requirement together.

A separate observer process can instead hold its observer, catalog, and one decoded N512 state: the formula minimum is 235,546,892,872 bytes before importer, manifest, output, allocator, and operating-system overhead. That is an ownership observation, not an admission: the current W3 allowlist refuses both layout 768 and 1024.

## Formula sources

- `crates/nsbu-solver/src/domain/resources.rs`: the first four ResourcePlan classes are retained `half_len * 576`, padded real `* 72`, padded half `* 48`, and retained `* 48`.
- `crates/nsbu-solver/src/integrators/attempt.rs` and `integrators/{method,kernel}.rs`: Cox--Matthews attempt storage gives 30,182,212,728 bytes at N512.
- `crates/nsbu-solver/src/spectral/{rotational.rs,w3/admission.rs,fft/avx.rs}` and `integrators/rhs.rs`: RHS, AVX workspace, and hypothetical extension of the existing W3 arithmetic ledger.
- `crates/nsbu-benchmarks/src/provider/{reduced.rs,parallel/admission.rs,parallel_reduced.rs}` and `evidence/p10/avx-w3-n256-integration-20260912/harness/src/cache.rs`: cached M768 force formula.
- `crates/nsbu-solver/src/diagnostics/conservative.rs` and `evidence/p10/avx-parallel-reduced-composite-7467e26/harness/src/observer.rs`: observer formula and its ten N1024 component fields.
- `crates/nsbu-solver/src/spectral/w3/admission.rs`: current closed cubic W3 set is only 6, 384, 512, and 576. It refuses 768 and 1024.

## Minimal future controls, not executed

A 768-only W3 admission candidate needs two independent controls before it could support any trajectory preflight:

1. A forward force control at retained N512, samples M768: build serial and W3 providers sequentially from identical inputs; compare all three retained coefficient words bit-for-bit and complete `ForceWork`; verify W3 identity is layout768, width3, forward; assert the exact formula reservation and refusal at one byte below it.
2. A bidirectional rotational control at retained N512 with padded768: build serial and W3 RHS/workspaces sequentially on a finite strict/Hermitian fixture; compare complete projected RHS coefficient words and work/transform accounting; verify layout768, width3, bidirectional identity; assert exact reservation and one-byte-under refusal.

Both controls should run in a bounded standalone owner with allocation accounting and no trajectory, observer, archive, resume, or acceptance path. They are deliberately separate because force-forward and RHS-bidirectional owners have different scratch and worker lifetimes. A successful control would validate only the stated implementation/resource contract, not the memory ledger, endpoint runtime, force accuracy, convergence, or a PDE window.
