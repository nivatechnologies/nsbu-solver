# Clock 2048 matched spatial screen

All three immutable scheduled snapshots passed exact source, plan, identity, coefficient/file hash, u128 clock, finite, Hermitian, and Nyquist checks. The comparisons use the exact shared schedule prefix derived from the unchanged full piecewise plan.

| Pair | L2 | H1 | Vorticity L2 | H1 allocation multiple | Vorticity allocation multiple |
|---|---:|---:|---:|---:|---:|
| N192 to N256 | `0.00010285921744338136` | `0.0768370280160634` | `0.07683695917166418` | `4.019890658269159` | `4.0225605598805885` |
| N256 to N384 | `1.887878376649154e-05` | `0.018559277762550327` | `0.01855926816447093` | `0.97096756109729` | `0.9716128190093408` |

The fine/coarse H1 error ratio is `0.2415408071050114`. The fine-pair squared H1 new-shell fraction is `0.9999997845322338`. Exact full/common/new-shell values and all relative/budget ratios are in `summary.json`.

This is one actual matched-trajectory spatial screen node, not an acceptance or PDE qualification claim.
