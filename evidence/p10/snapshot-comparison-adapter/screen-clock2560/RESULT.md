# Clock 2560 matched spatial screen

All three immutable scheduled snapshots passed exact source, plan, identity, coefficient/file hash, u128 clock, finite, Hermitian, and Nyquist checks. The comparisons use the exact shared schedule prefix derived from the unchanged full piecewise plan.

| Pair | L2 | H1 | Vorticity L2 | H1 allocation multiple | Vorticity allocation multiple |
|---|---:|---:|---:|---:|---:|
| N192 to N256 | `0.00010332730613999712` | `0.07715447169357514` | `0.07715440250704263` | `3.9858848508768823` | `3.988529469075018` |
| N256 to N384 | `1.896997741930139e-05` | `0.018642519775438238` | `0.01864251012774405` | `0.9630930006944375` | `0.9637323741139554` |

The fine/coarse H1 error ratio is `0.24162591443148523`. The fine-pair squared H1 new-shell fraction is `0.999999778624133`. Exact full/common/new-shell values and all relative/budget ratios are in `summary.json`.

This is one actual matched-trajectory spatial screen node, not an acceptance or PDE qualification claim.
