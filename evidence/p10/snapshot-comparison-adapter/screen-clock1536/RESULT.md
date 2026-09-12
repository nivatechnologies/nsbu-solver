# Clock 1536 matched spatial screen

All three immutable scheduled snapshots passed exact source, plan, identity, coefficient/file hash, u128 clock, finite, Hermitian, and Nyquist checks. The comparisons use the exact shared schedule prefix derived from the unchanged full piecewise plan.

| Pair | L2 | H1 | Vorticity L2 | H1 allocation multiple | Vorticity allocation multiple |
|---|---:|---:|---:|---:|---:|
| N192 to N256 | `9.574962520922876e-05` | `0.071555426881423` | `0.07155536282182892` | `4.048950585908704` | `4.051639041297283` |
| N256 to N384 | `1.7569015247029432e-05` | `0.0172775400539527` | `0.017277531124791533` | `0.9776463315687084` | `0.978295847108543` |

The fine/coarse H1 error ratio is `0.24145673929922767`. The fine-pair squared H1 new-shell fraction is `0.9999998168936041`. Exact full/common/new-shell values and all relative/budget ratios are in `summary.json`.

This is one actual matched-trajectory spatial screen node, not an acceptance or PDE qualification claim.
