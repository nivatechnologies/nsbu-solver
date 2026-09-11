# Exact-v2 regional analytical tracking

This increment adds sampled geometric classification to the complete global
velocity, ordered gradient/Hessian and vorticity errors for all six actual
exact-v2 family branches. The [public guide](../../../docs/V2_REGIONAL_TRACKING.md)
defines its ownership, admission, coverage semantics and scientific limits.

Three regional harness tests, the four-test established global tracking
regression, one allocation executable and the release example pass. A separately
evolved but identical family checks all 72 global findings at all three clocks
for exact equality with the existing consumer. Its retained independent signed
full-complex Fourier oracle and 120-digit analytical fixture remain green.

At endpoint 1/8192, the N12/h16 CM ordered-Hessian class counts and RMS errors are
Core 15/1.723373873675785e-4, Annulus 96/3.008181608432012e-4,
InteriorOutsideNominal 68/4.940621458484716e-4, Collar
336/2.071234674533215e-3 and Exterior 1213/1.713518266755864e-4. Counts sum to
1,728 because `SpatialRegion` classifies each sample once. This does not imply
completeness for separately selected nominal coverage sets, which can overlap.

Nightly branch coverage across the three new production files is 209/227 lines
(92.07%) and 7/8 branches (87.50%). Across those files plus the focused tests,
allocator executable and example, maxima are CC8, cognitive13, Halstead57.1429, RCA ploc268 and a maximum actual file length of 281 lines. Maximum per-function CRAP is 9. Focused strict Clippy,
formatting and the exact workspace Rustdoc gate pass.

Admission and execution allocate nothing. Construction used 20,135,360 bytes
within the 25,185,960-byte joint reservation. Three reports add 124,416
classifications and magnitude visits and conservatively charge 15,925,248 root
iterations. Refused foreign/stale/exhausted requests charge work but preserve
state and schedule; terminal family failure also preserves state. Missing
sampled classes remain `NoSamples`, never a measured zero.

An initial source-matched quality report measured CRAP26.125 for combined
quantity reduction/dispatch. The final source separates global reduction and
component dispatch without changing results and passes at CRAP9. An initial
stable-toolchain coverage command refused the nightly branch option; the stored
coverage was regenerated successfully with nightly-2026-03-03. The earlier
hosted Rustdoc failure at `dad47c4` from undocumented public oracle fields remains
part of the parent run record; this branch incorporates the subsequent comment
fix and passes `RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked`.

The analytical reference and actual FFT arithmetic remain binary64 without a
current-grid arithmetic bound. Pressure/gauge, continuous or regional supremum
bounds, convergence, arbitrary nominal-set completeness and PDE qualification
remain open. Accepted concentrating windows remain zero.
