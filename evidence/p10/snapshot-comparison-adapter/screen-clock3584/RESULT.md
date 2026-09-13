# Clock 3584 matched spatial screen

All three immutable scheduled snapshots passed exact source, plan, identity, coefficient/file hash, u128 clock, finite, Hermitian, and Nyquist checks. The comparisons use the exact shared schedule prefix derived from the unchanged full piecewise plan.

| Pair | L2 | H1 | Vorticity L2 | H1 allocation multiple | Vorticity allocation multiple |
|---|---:|---:|---:|---:|---:|
| N192 to N256 | `0.0001042987736575404` | `0.0778132351183085` | `0.07781316522141823` | `3.8980877139634265` | `3.9006541392406344` |
| N256 to N384 | `1.91590312321375e-05` | `0.018815161999823405` | `0.018815152249266574` | `0.9425536224141327` | `0.9431745395986324` |

The fine/coarse H1 error ratio is `0.24179899436408897`. The fine-pair squared H1 new-shell fraction is `0.9999997656031643`. Exact full/common/new-shell values and all relative/budget ratios are in `summary.json`.

This is one actual matched-trajectory spatial screen node, not an acceptance or PDE qualification claim.
