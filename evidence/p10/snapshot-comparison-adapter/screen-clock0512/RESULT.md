# Clock 512 matched spatial screen

All three immutable scheduled snapshots passed exact source, plan, identity, coefficient/file hash, u128 clock, finite, Hermitian, and Nyquist checks. Both comparisons use the exact `[0,512)` h64 prefix derived from the unchanged full piecewise plan.

| Pair | L2 | H1 | Vorticity L2 | H1 allocation multiple | Vorticity allocation multiple |
|---|---:|---:|---:|---:|---:|
| N192 to N256 | `6.5958758267507075e-6` | `4.933147217475805e-3` | `4.933142808138362e-3` | `4.095574964055182` | `4.098285356862772` |
| N256 to N384 | `1.2096041932421298e-6` | `1.190327045752841e-3` | `1.1903264313918459e-3` | `0.988227809391572` | `0.9888821775622612` |

The fine-pair H1 error is `0.24129161228679247` of the coarse-pair error and remains narrowly below its allocation. The fine-pair vorticity error is likewise below allocation. Newly resolved shells contain `0.9999999983509964` and `0.9999999992032038` of squared H1 error for the coarse and fine pairs. Exact full/common/new-shell values and relative/budget ratios are in `summary.json`.

This is one actual matched-trajectory spatial screen node. It is not an acceptance or PDE qualification claim; seven requested positive nodes remain part of the aggregate screen (clock 1024 is archived separately).
