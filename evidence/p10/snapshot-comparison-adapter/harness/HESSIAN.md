# Optional ordered-Hessian diagnostic

Passing `--ordered-hessian` after the existing three adapter arguments selects a separate,
non-acceptance output. Existing invocations and `NormOutput` serialization remain unchanged. The
option is restricted to already valid `MATCHED_SPATIAL` manifest pairs, so all normal manifest,
plan, state, hash, identity, clock, finite-spectrum, Hermitian, Nyquist, and memory-cap checks run
before this diagnostic.

For each velocity component and Fourier mode, the screen accumulates all nine ordered second
derivatives. Their squared Frobenius magnitude is evaluated without allocating as

```text
|u_hat|^2 (kx^2 + ky^2 + kz^2)^2.
```

Half-plane Parseval weights restore the omitted conjugate modes, and strict Nyquist planes remain
excluded. A scaled sum-of-squares accumulator avoids first squaring a large representable modal
amplitude. `rms` is the physical volume-average Frobenius norm implied by normalized Fourier
coefficients. `l2` multiplies that value by the square root of the physical domain volume.

The output reports full-fine-band coarse/fine difference, fine absolute scale, and their ratios,
plus exact state hashes, plans, identities, profiles when present, sources, backends, execution
bindings, admission guards, and clocks. A zero fine denominator is serialized as `null`. The
diagnostic has no assigned error budget, makes no acceptance decision, and supports no pointwise,
time-supremum, finest-grid, or continuum claim.
