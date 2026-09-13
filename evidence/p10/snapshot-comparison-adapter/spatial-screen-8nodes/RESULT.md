# Eight-node matched spatial screen

All eight positive scheduled nodes passed exact source, plan, evolution, identity, coefficient/file hash, u128 clock, finite, Hermitian, and Nyquist checks in their source evidence. The aggregate `summary.json` preserves both adjacent pairs at every node, including full, common-band, and newly-resolved L2/H1/vorticity/divergence values; fine absolute norms; relative values; allocated-budget multiples; and fine/coarse reductions.

| Clock | Fine L2 multiple | Fine H1 multiple | Fine vorticity multiple | Fine/coarse H1 | Fine new-shell H1 squared fraction |
|---:|---:|---:|---:|---:|---:|
| 512 | 0.275983333 | 0.988227809 | 0.988882178 | 0.241291612 | 0.999999999 |
| 1024 | 0.274429682 | 0.983345253 | 0.983997723 | 0.241373653 | 0.999999949 |
| 1536 | 0.272756941 | 0.977646332 | 0.978295847 | 0.241456739 | 0.999999817 |
| 2048 | 0.270947881 | 0.970967561 | 0.971612819 | 0.241540807 | 0.999999785 |
| 2560 | 0.268979483 | 0.963093001 | 0.963732374 | 0.241625914 | 0.999999779 |
| 3072 | 0.266823853 | 0.953744586 | 0.95437603 | 0.241712006 | 0.999999772 |
| 3584 | 0.264445718 | 0.942553622 | 0.94317454 | 0.241798994 | 0.999999766 |
| 4096 | 0.261799652 | 0.929019765 | 0.929626818 | 0.241886797 | 0.999999758 |

Every N256 to N384 H1 and vorticity result is below its allocated relative budget, and every fine-pair L2/H1/vorticity error decreases from the corresponding N192 to N256 error. The narrowest margin occurs at clock 512. Coarse-pair budget exceedance is retained as measured context and is not a ladder failure under the reviewed sequence gate.

This is an actual matched-trajectory spatial screen through clock 4096. It is not an acceptance or PDE qualification claim.
