# V2 force-grid refinement discriminator

This source-bound force-only study evaluates the unchanged prescribed force at
M24, M48, M96 and M192, with 12 workers, and retains each result on the same
N24 grid. It samples exact clocks 2047 and 4096 from source commit
`31e99a17f97aec2ee18b26c67f8be88a0e931088`.

For each clock, it reports full-band norms for each raw force and each adjacent
difference (M24/M48, M48/M96, M96/M192). It then applies the N24 Leray
projector to the same retained fields in place and repeats those reports.
Coefficient SHA-256 payload digests, actual provider charges and the full raw
output are retained.

Providers are constructed, evaluated and dropped sequentially per clock. The
peak reservation is 482,053,416 bytes: M192 provider 480,611,624 + four N24
vector fields 1,437,696 + 4,096 explicit harness allowance, under the 1 GiB
cap. The declared bound is 8 evaluations, 24 transforms, 2,086,456,320 provider
work units, 1,467,648 comparison visits and 179,712 projected component visits.
The completed run took 39.77 wall seconds (365.22 user seconds), had 448,460 KiB
maximum RSS and exited zero under a 900-second timeout. Actual provider work is
reported separately in the raw output and summary.

At clock 2047, projected adjacent L2 differences are 1854.1746972397223,
214.558170886227 and 6.849576791047435 for M24/M48, M48/M96 and M96/M192.
At clock 4096 they are 1849.3879473242673, 215.07799780480406 and
6.875573117434549. These are empirical prescribed-force sampling observations.
They do not compare a nonlinear operator or full RHS, establish force convergence,
qualify a trajectory, or accept a PDE window.

Reproduce from a checkout at the recorded commit with:

```sh
/usr/bin/time -v timeout 900s cargo run --release \
  --manifest-path evidence/p09/v2-force-grid-refinement/harness/Cargo.toml
```
