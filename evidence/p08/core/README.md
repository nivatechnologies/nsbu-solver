# P08 independent diagnostics: first increment

This is a **partial P08 implementation**, not a completed package or a qualified PDE window. The source and configuration hashes in [summary.json](summary.json) identify the tested increment. Remaining P08 work includes reporting, local/directional diagnostics, accepted-history pressure and balance comparisons, and window review with explicit missing-channel handling.

The Rust library now provides complete fine-band comparisons, volume-average L2/H1/curl/divergence norms, unremoved mean errors, exact-clock quintic Hermite reconstruction, independently formed conservative products and physical pressure on a separately reserved double grid, reconstructed PDE residuals, Parseval balance terms, and Simpson balance quadrature. Diagnostics borrow field data and cannot replace or reset integrated state. The CLI still supports help/version only.

## Scientific checks

- Full-band tests retain newly resolved modes. A forcing negative control has nearly identical cropped 8/12-grid inputs but a full-band discrepancy of 1.0; cropped agreement cannot establish force resolution.
- Polynomial reconstruction reproduces every degree through five. A sixth-degree control has zero nodal defects but nonzero off-node defects, including the exact values ±3/512 on the unit interval. Halving the interval reduces this control's defect by 32.
- Gradient force changes physical pressure without causing projected acceleration. Taylor–Green pressure retains doubled modes outside the velocity grid. A separate three-dimensional trigonometric fixture has exact stretching -1/2, verified against independent symbolic integration.
- Balance quadrature integrates cubic rates to roundoff and exhibits fourth-order composite refinement on a quartic rate. Compensated signed reductions retain a small term between large cancelling contributions. These synthetic tests are not PDE trajectories.
- Both CM and HO evolve the smooth cyclic-sine fixture from rest to 1/32. Reconstruction uses the final three **accepted states** and derivatives formed through the independent conservative path. All history and diagnostic buffers are preflighted; the diagnostic path makes exactly twelve bounded force requests per trajectory, with nine scalar transforms each. No analytical velocity is assigned to a state.
- Four macro intervals use 4096, 2048, 1024, and 512 ticks at exponent -20. Every run probes the same physical time 511/16384 and eight additional points strictly between all full/two-half integrator stage clocks. Common-probe defects decrease approximately cubically for this geometry. Maxima over the eight samples on successively shrinking final reconstruction intervals decrease approximately quartically. The latter are **not** maxima over a common interval, rigorous suprema, or slab-wide bounds. See [accepted-history.log](accepted-history.log).

## Verification scope

The full test/coverage run includes all scientific studies. Three named long trajectory studies are omitted only from mutation reruns, where focused operator, provider, transaction, reconstruction and analytic-fixture tests exercise every production mutation. No production source is excluded from mutation, and no equivalent-mutant exemption is used. Unviable mutants are listed separately from caught mutants; timeouts are never counted as kills.

The quality scope includes owned Rust source and tests. Physical file length includes comments and blank lines. Dead/redundant-code findings describe strict linting, clone detection and review, not a formal proof of semantic minimality. Executable-line and instrumented-branch coverage are the coverage gates; raw LLVM region and instantiation statistics are retained without relabelling them as 100%.

Published checkout/home prefixes are normalized. Raw execution records remain under `work/`. Reviewed design files, exact-v2 input bytes, historical results and their hashes are unchanged. The standalone package has no private Niva dependency.

The source-matched [hosted Python/repository run](hosted-python-ci.json) passed.
Hosted Rust verification is pending. P08 remains incomplete.

The subsequent [reporting and accepted-history balance increment](../reporting/README.md)
extends these diagnostics. This core report retains the evidence for its own
earlier source snapshot.
