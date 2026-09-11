# Exact-v2 nested physical sampling evidence

This increment measures the same admitted exact-v2 family on strictly nested
`12^3`, `24^3`, and `48^3` periodic physical lattices. Each successive axis is
both larger and an integer multiple of its predecessor. The consumer retains
velocity, gradient, ordered Hessian, and vorticity statistics for all five
state pairs at all three layouts. Each absolute, relative, and finer-state
reference maximum records its layout, first x-major/z-fast linear index, and
three-dimensional index. `reference_peak` is the finer comparison state's
sampled magnitude, not an analytical reference.

The release-mode diagnostic completed three clocks and emitted all 180 raw
level records in `raw/numerical.log.gz`. It took 165.72 seconds wall time and
49,000 KiB maximum process RSS. The allocation executable measured 29,520,704
constructed bytes within the 34,644,696-byte joint reservation, with zero
admission allocations and zero steady-state allocations; its 144,612 KiB
maximum process RSS includes the test harness and runtime and is not the
reservation. Complete grids always produce `Measured`; the helper's explicit
`NoSamples` result was tested on an empty stream and never substitutes a zero.

Admission tests reject unequal, decreasing, anisotropic, and increasing but
non-divisible (`12/20/48`) layouts, insufficient joint storage, invalid floors,
work overflow, wrong identity/clock, and exhausted attempts. A separate direct
signed Fourier sum checks selected values on every layout, including
negative-z conjugate reconstruction and an excluded Nyquist mode. The consumer
does not evolve the family again and does not inject a reference solution.

The initial instrumented run exposed a branch-coverage gate failure: 36/50
branches (72.00%) when maintained source and integration-test assertion code
were combined. The correction added direct physical-plan cap and retained-band
controls, exercised both witness variants, and simplified two private extrema
invariants already guaranteed by `PhysicalComparison` and admitted lattice
bounds. On the final source, maintained library coverage is 432/462 executable
lines (93.51%) and 26/32 branches (81.25%). The broader diagnostic including
integration-test assertion code is 800/830 lines (96.39%) and 36/46 branches
(78.26%), below 80%. The complete maintained-source release gate therefore
remains pending the combined hosted coverage run; no assertion-only tests were
added to inflate this focused number.
Maximum CRAP is 13.125, all-node Halstead difficulty is 45.55, maximum
per-function cyclomatic complexity is 13, and maximum per-function cognitive
complexity is 19. Formatting, strict focused Clippy, and warning-denied
workspace Rustdoc pass.

The raw release-mode metric log predates only the coverage-control test edits
and the equivalent private extrema counter cleanup; final-source LLVM coverage
reran all three numerical tests and the allocation executable successfully.
The artifacts support a bounded sampling-sensitivity diagnostic. They do not
qualify a window, establish convergence, define a channel budget, or change an
acceptance threshold. Pressure remains outside this four-quantity study.
