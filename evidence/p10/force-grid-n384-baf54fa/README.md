# N384 M384 versus M512 exact-v2 force sampling rank

This isolated, source-bound diagnostic compares physical force sampling M384
and M512 at fixed retained N384 and exact clock 4096. It uses the opt-in
`RustFft6_4_1AvxFma` backend and `ParallelReducedV2Force` with 32 workers.

The outputs rank M384 versus M512 for a possible future N384 pilot. They do not
establish force sufficiency, a force error budget, reduced-arithmetic
qualification, nonlinear error, or PDE trajectory qualification.

The two samples run sequentially under the harness's 96 GiB cap and an outer
timeout. Because long endpoint jobs were active, sampling uses `nice -n 10` and
all timings are labeled contention-affected. Raw spectra remain in the ignored
local `work/` directory; only hashes and small evidence are tracked.

## Identity and admission

- Base source: `baf54face1194447496f58d60d314e58a1329146`
- Source-bound harness/analyzer: `ca33892df5c5dfd7d15a5aeeb82382791dbfddfd`
- Harness SHA-256: `a18dcd9fc06b2eac0ad55c7d5925f69971994d8b3d913f6f691c2a05c6f0001f`
- Binary SHA-256: `2f7e6cc68ba4b1bcd8e5cd431b32d6d527b9476ce2533e18800c972b5a5673d0`
- Clock: quantum `2^-20`, target 8192, elapsed 4096, remaining 4096

Each retained artifact is 1,366,032,632 bytes including its header. Exact
admitted peaks were 6,460,723,176 bytes for M384 and 11,430,567,912 bytes for
M512, including the AVX catalog, provider storage, two retained outputs and
artifact overhead. Both are below the 103,079,215,104-byte cap.

Two endpoint processes were active as authorized. Before M384 they occupied
45,168,516 and 63,334,436 KiB RSS and averaged 1174% and 708% CPU. Before M512
they retained the same RSS and averaged 1232% and 736% CPU. Available memory was
405.3 and 405.5 GB respectively. All timings are contention-affected.

## Results

| M | Construction (s) | Force (s) | Wall | Max RSS (KiB) | Coefficient SHA-256 | Artifact SHA-256 |
|---:|---:|---:|---:|---:|---|---|
| 384 | 2.139992454 | 15.841049036 | 0:36.12 | 4,881,408 | `52abb299c0f250007f3cc9da621aee3fdfe5ec6ffdae0825a97c7304fcc398cf` | `e265614b1d07b74b96f61d40d979874d20b549cb5ffd3c990528cd7b4c74fcf0` |
| 512 | 4.739267761 | 26.165507307 | 0:47.42 | 9,735,168 | `e56c8a5f8275fc9d8ef8a7a61b63c1801f0d87f2c9445ac4217bb377494b055d` | `1a5f92065b9bebdfa218bd024e742e4483a3d525a951c4cad54d20671da476f8` |

Both spectra passed full finite, Hermitian and Nyquist validation. Since the
retained layout is fixed, `common == full` and newly-resolved norms are zero.

| Channel | Absolute difference | Fine M512 norm | Difference/fine |
|---|---:|---:|---:|
| L2 | 2.2021394902342624 | 3.4414613597929915e3 | 6.39884996519828422e-4 |
| H1 | 2.8847970275050516e3 | 5.56561979029213e5 | 5.18324487874087005e-3 |
| Vorticity L2 | 1.6409817135756357e3 | 5.565435701020226e5 | 2.94852335330155662e-3 |
| Divergence L2 | 2.3725994310323226e3 | 2.9406759100543e3 | 8.06821119906583784e-1 |

The force itself is not divergence-free, so raw divergence is a comparison
channel rather than a zero-validity target. Full-band Leray projection,
retaining zero mode, gives `P(delta f)` L2 `1.3175466287891247`, H1
`1.6409822421544059e3`, vorticity L2 `1.640981713575636e3`, and divergence L2
`2.377852353868065e-13`.

The ranking-only Stokes response at `T=4096/2^20=1/256` gives L2
`9.108013397637507e-7` and H1 `1.0809685444601123e-3`. It applies
`(1-exp(-|k|^2*T))/|k|^2` to `P(delta f)`, with zero-mode factor `T`.

This two-grid result favors M512 over M384 for the next N384 pilot: raw
H1/fine-H1 is about 0.518%, and the projected difference retains H1
`1.6409822421544059e3`. It does not establish that M512 is sufficient. M768 was
not run because the approved pair gives a clear ranking and further work needs
root review.

Exact commands and raw console/time output are in `summary.json` and `logs/`.
The untracked raw archive is
`/mnt/niva-array/nsbu-solver/work/p10-force-grid-n384-20260912/evidence/p10/force-grid-n384-baf54fa/work/`.
