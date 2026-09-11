# Test-only reference-oracle phase cache

This bounded increment speeds the independent exact-v2 reference-tracking test
oracle. It changes no production code. The oracle still reconstructs every
requested quantity with a direct signed Fourier sum and does not call the
production FFT, transfer, derivative, or reference-tracking implementation.

The original control computed
`sin_cos(sum(TAU * signed_mode[axis] * point[axis]))` for every mode and sample.
The final oracle independently computes one phase table for each axis and uses
the product of the three stored factors inside the same modal sum. No
coefficients or waves are cached.

The legacy direct-angle implementation remains compiled only for tests. On the
existing N=4, N=8, and N=12 branches, it compares every velocity, gradient,
ordered-Hessian, and curl component at an axis point, the sampled 5/12 cutoff
collar, and the mixed-coordinate collar point (1,2,3)/12. Each point check visits every signed mode and
explicitly observes negative-z modes. Component comparisons use the chosen
roundoff allowance
`128 * f64::EPSILON * modal_terms * sum_abs_term_magnitudes`; an exactly zero
magnitude requires bitwise equality. Phase factors use `32 * f64::EPSILON`.
These are focused comparison bounds, not rigorous platform-libm certification.
The pre-existing tracking assertion remains exactly
`abs(measured - oracle) < 2e-10 * (1 + oracle)`.

The N=12/12-sample basis preflight reserves 6,336 bytes of values, 120 bytes of
inline `Basis`/`Vec` metadata, and 192 bytes of conservative allocator overhead:
6,648 bytes total against an 8 KiB allowance. Overflow or excess size is refused
before allocating the vectors. The enclosing test retains its existing 16 MiB
joint-cap headroom.

The identical release test and its six branch inputs were measured with a timer
around only the independent oracle call. The legacy calls totaled 0.977051453 s;
the cached calls totaled 0.719379027 s, a measured 1.358187x speedup and 26.372%
reduction. Whole focused-test wall time was 36.95 s before and 35.88 s after;
PDE evolution and analytical reference evaluation dominate that test. These are
single local measurements, not a general runtime or statistical performance
claim. Timing instrumentation was removed before the final gates.

The final four-test numerical harness passes. Focused coverage is 502/506 lines
(99.209%), 10/12 branches (83.333%), and 47/47 functions. Maxima are CC10,
cognitive21, Halstead difficulty39.8919, physical-file292, and CRAP10. Focused
strict Clippy and formatting pass, as does the final whole-workspace Rustdoc
build with warnings denied. The compressed raw reports below are bound to the
three changed source files by [source-sha256.json](source-sha256.json).

No pressure-oracle optimization, production behavior, sample grid, clock,
tolerance, fixture, or scientific acceptance claim is included.
