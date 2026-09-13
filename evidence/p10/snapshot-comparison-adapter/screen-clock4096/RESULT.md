# Clock 4096 matched spatial screen

All three immutable scheduled snapshots passed exact source, plan, identity, coefficient/file hash, u128 clock, finite, Hermitian, and Nyquist checks. The comparisons use the exact shared schedule prefix derived from the unchanged full piecewise plan.

| Pair | L2 | H1 | Vorticity L2 | H1 allocation multiple | Vorticity allocation multiple |
|---|---:|---:|---:|---:|---:|
| N192 to N256 | `0.00010480243350975437` | `0.07815477879146271` | `0.0781547085263208` | `3.8407215301566042` | `3.8432297260747794` |
| N256 to N384 | `1.9256934063732502e-05` | `0.018904609111515817` | `0.018904599307731886` | `0.9290197651344548` | `0.9296268180066143` |

The fine/coarse H1 error ratio is `0.24188679699239166`. The fine-pair squared H1 new-shell fraction is `0.9999997583898618`. Exact full/common/new-shell values and all relative/budget ratios are in `summary.json`.

This is one actual matched-trajectory spatial screen node, not an acceptance or PDE qualification claim.
