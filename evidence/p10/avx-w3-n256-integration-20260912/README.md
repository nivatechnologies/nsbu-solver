# N256 separate-owner W3 actual gate

This source-bound experiment compares the unchanged serial N256/M384 accelerated
attempt with an explicit W3 path. The W3 path owns separate fixed three-worker
FFT pools inside the RHS rotational workspace and reduced-force provider. The
observer remains serial and unchanged. Existing constructors, traits, defaults,
and scheduled binaries do not select this path.

The four whole attempts ran sequentially in serial/W3/serial/W3 order from the
same rest state while the two existing endpoint processes remained active. The
serial integration median is 198.353954388 s and W3 is 139.6863635165 s, a
29.5772% reduction. Each pair independently passes 25% (26.2586%, 32.7268%).
Charging the measured observer medians at the scheduled 1/16 frequency gives
208.9254077531 s serial and 150.6606856465 s W3, a 27.8878% reduction. This
passes the gate by 2.8878 percentage points.

All four attempts have the exact same state SHA-256, indicators, work record,
cache hit/miss count, and balance sample. Every measured attempt reports zero
allocations, deallocations, and reallocations. W3 accounting records 36 RHS
triplets, the five cached reduced-force forward triplets, and 12 scalar pressure
FFTs. Preflight charges 2,733,911,936 additional RHS bytes and 1,827,942,144
additional force bytes, exactly 4,561,854,080 bytes total.

This is an actual startup-step performance and numerical control, not arithmetic,
spatial, temporal, quadrature, PDE-window, or later-state qualification. The
host was contended by two active endpoints, so paired ratios and the unchanged
observer timings provide the load context. These runs bind pre-split source
`8f579375d2d5ad79a573880393c05a297acf98d5`. The binary's embedded `RUN_SOURCE`
label used the intended short prefix with an invalid suffix; `source-commit.txt`
records the resolvable source object, while the original stdout and binary hash
remain unchanged.

The final integrated source `f13c29c9ae91d0b8cf7a790132deb9bd076911c0`
produced the same state hash, indicators, work, cache record, and balance sample
as all four pre-split controls. Its integration time was 139.318356566 s, within
0.368 s (0.264%) of the earlier W3 median; observer time was 175.143311236 s and
total time was 314.604717602 s. It again reported zero steady allocations and
the exact 36 RHS triplets, five force triplets, and 12 scalar pressure FFTs.
This confirms the FFT-module and quality splits did not change the measured W3
arithmetic or materially change startup-step performance.
