# Numerical conventions

This guide collects conventions needed to interpret the implemented library and
its diagnostics. It does not expand the mathematical specification or certify a
numerical result. The exact benchmark identity remains the
[`similarity-mms-v2` manifest](../benchmarks/similarity-mms-v2.json), and the
limits on scientific interpretation are in [scientific scope](SCIENTIFIC_SCOPE.md).

## Equation, domain, and state

The library evolves three velocity components on a periodic three-dimensional
domain with positive viscosity and prescribed force:

```text
partial_t u + (u dot grad)u = -grad p + nu Laplacian(u) + f(x,t)
div u = 0
```

The state is stored as a real-to-complex half spectrum. A `Domain` validates
the physical grid and viscosity, while `Layout` fixes the retained half-spectrum
shape. Inputs must be finite, obey Hermitian symmetry, and satisfy the strict
Nyquist rules enforced by `validate_spectrum`. Fourier-facing public values use
the crate's binary64 `Complex64` coefficient type.

For a grid with M points, the forward transform uses
`u_hat(k) = (1/M) sum_j u(x_j) exp(-i k dot x_j)` and the inverse uses
`u(x_j) = sum_k u_hat(k) exp(+i k dot x_j)` without a further factor.
Physical wave numbers are `k_i = 2*pi*m_i/L_i`. The half spectrum retains
nonnegative last-axis modes; negative last-axis modes follow by conjugacy.
Any mode on a retained Nyquist plane is zero. Strict-band transfer
between retained and padded representations copies the common band without
amplitude rescaling and zeroes the other destination coefficients. Quadratic
products use three-halves padding. The exact indexing, normalization, and
transfer behaviour are documented with the implementation in
[`spectral`](../crates/nsbu-solver/src/spectral/mod.rs) and
[`transfer`](../crates/nsbu-solver/src/spectral/transfer.rs); callers should not
substitute an alternative FFT normalization or retain omitted Nyquist values.

## Pressure, norms, and diagnostics

The modal operator's modified pressure is `pi = p + |u|^2 / 2` and has zero
spatial mean. The benchmark scalar evaluator returns *raw* pressure; remove its
spatial mean before a zero-gauge comparison. This distinction matters when
comparing a physical-pressure diagnostic with the manufactured reference.

Library norms and balance quantities are physical **volume averages**, not
domain integrals. Full-band comparisons retain all fine modes and preserve mean
differences; they do not align, recenter, or suppress modes to improve an error.
Directional tails overlap and must not be added as if they partitioned the
spectrum.

Reported sampled maxima, local errors, regional volumes, residuals, Hermite
reconstructions, and quadrature values are finite floating measurements.
They are not continuous-time/spatial enclosures or certified error bounds.
An absent sample is represented explicitly, rather than as zero error. The
diagnostic API's module comments are the authoritative per-measurement details:
[`norms`](../crates/nsbu-solver/src/diagnostics/norms.rs),
[`comparison`](../crates/nsbu-solver/src/diagnostics/comparison.rs),
[`sampling`](../crates/nsbu-solver/src/diagnostics/sampling.rs), and
[`quadrature`](../crates/nsbu-solver/src/diagnostics/quadrature.rs).

## Exact time and local acceptance

Physical time is represented by an exact integer tick count times `2^exponent`.
`TickClock` keeps this exact geometry separate from a force provider's binary64
conversion. Requested integration intervals must admit the required quarter
stage clocks; a duration conversion refuses underflow, overflow, or lost low
bits rather than silently rounding.

A CM or HO attempt compares one full step with two half steps on the exact
requested interval. The resulting velocity/vorticity discrepancies are local,
empirical indicators. A locally accepted attempt is eligible for a transactional
commit; it is not evidence of global accuracy, convergence order, a provenance
claim, or a PDE window. The methods differ in bounded RHS work: CM uses twelve
calls and HO uses fifteen for a complete full/two-half attempt. See
[`AttemptWorkspace`](../crates/nsbu-solver/src/integrators/attempt.rs) and the
working test-oriented commands in [usage](USAGE.md).

## Benchmark-specific interpretation

The first concentrating case is manufactured prescribed forcing, not a
reproduction of the source paper's annular-pulse construction and not a
finite-time blow-up proof. Every comparison trajectory begins at exact rest at
time zero and evolves independently; the analytical field is available to the
verifier but must never replace the integrated state.

For this case the unit periodic cube has `nu = 1`, `T_star = 1/128`, and
target times `t_k = T_star * (1 - 2^(-k))`. The resource-resolution screen and
the coarse diagnostic results are not convergence acceptance tests. The
[usage guide](USAGE.md) records the remaining comparison families and the
evidence needed before a concentrating result may be claimed.
