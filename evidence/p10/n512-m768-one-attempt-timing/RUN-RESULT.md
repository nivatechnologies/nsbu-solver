# N512/M768 one-attempt executed result


The single authorized attempt ran from `2026-09-13T13:15:59Z` through
`2026-09-13T13:52:49Z` and exited zero. `/usr/bin/time` measured `36:49.71` wall time,
`155,283,456` KiB maximum RSS, zero swaps and one major fault under the exact 256-GiB
address-space cap. The harness measured `2067.912943179` seconds in the integrator and
`1969.460301335` seconds in 12 RHS calls, with the expected five cache misses and seven
hits. It consumed 4,690,292,918 force work units and 135 scalar force transforms, and
reported zero steady allocations.

The local error ratios were `1.5877938231527898e-08` in L2 and
`4.324683895723379e-08` in H1, so a local accepted token was present. The token was
intentionally dropped: `committed`, `published` and `qualification` are all false. This
is one-attempt timing and resource evidence. It supplies no trajectory, refinement,
residual, PDE qualification or accepted-window result.

All recorded owner, timeout and worker PIDs were absent after exit, no matching process
remained, and post-exit `MemAvailable` was `382,115,372` KiB. The result SHA-256 is
`2651aefa9055cc19c8c2fde0367f6beea57de592ba1cf1bea7970e53778a8102`.
`raw/one-attempt-20260913/validation.json` records the exact checks and
`raw/one-attempt-20260913/SHA256SUMS` binds every raw run artifact.
