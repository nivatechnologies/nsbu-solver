# Clock-dependent sampler profile

This lightweight diagnostic leaves the frozen numerical source unchanged. A 52-line external
harness evaluates the same deterministic Cartesian lattice at exact clocks 512 and 4096, separately
measuring `scalar::evaluate`, full `fields::reference::evaluate`, and regional `classify`. It sums
the existing `RootReport.iterations` counters and accumulates a black-box checksum so evaluations
cannot be removed. Each run is single-threaded and limited to 60 seconds.

The matched 32-cubed runs completed:

| operation | clock 512 seconds | clock 4096 seconds | 4096/512 | iterations 512 | iterations 4096 | iteration ratio |
|---|---:|---:|---:|---:|---:|---:|
| scalar velocity | 0.007203604 | 0.006920037 | 0.9606 | 45710 | 45654 | 0.9988 |
| full reference derivatives | 2.664591287 | 2.231349185 | 0.8374 | 45710 | 45654 | 0.9988 |
| regional classification | 0.003752675 | 0.003628593 | 0.9669 | 16238 | 16182 | 0.9966 |

An exploratory 96-cubed endpoint run also completed: scalar 0.1424 seconds, full reference 42.5800
seconds, classification 0.0872 seconds, and 1297134 scalar/reference root iterations. The clock-512
96-cubed run hit its 60-second limit before printing a record, so it is not a matched quantitative
comparison and makes no iteration claim.

The matched result argues against endpoint inflation in safeguarded root iteration counts or
single-thread pointwise scalar/reference/classification time. It does not identify the full-run
cause. The full projection has no phase timings, uses 32 threads and 1024-cubed points, and ran amid
different host activity. No evidence here establishes subnormal sensitivity, host contention, or a
specific slow phase. The endpoint numerical diagnostic remains unresolved.
