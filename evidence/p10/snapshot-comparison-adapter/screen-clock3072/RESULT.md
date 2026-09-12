# Clock 3072 matched spatial screen

All three immutable scheduled snapshots passed exact source, plan, identity, coefficient/file hash, u128 clock, finite, Hermitian, and Nyquist checks. The comparisons use the exact shared schedule prefix derived from the unchanged full piecewise plan.

| Pair | L2 | H1 | Vorticity L2 | H1 allocation multiple | Vorticity allocation multiple |
|---|---:|---:|---:|---:|---:|
| N192 to N256 | `0.00010380711084182898` | `0.07747983624424144` | `0.0774797667068516` | `3.945789332458535` | `3.9484002155660107` |
| N256 to N384 | `1.906338611754727e-05` | `0.01872780664606547` | `0.018727796947544218` | `0.9537445857826685` | `0.9543760295575144` |

The fine/coarse H1 error ratio is `0.24171200603766616`. The fine-pair squared H1 new-shell fraction is `0.9999997723302582`. Exact full/common/new-shell values and all relative/budget ratios are in `summary.json`.

This is one actual matched-trajectory spatial screen node, not an acceptance or PDE qualification claim.
