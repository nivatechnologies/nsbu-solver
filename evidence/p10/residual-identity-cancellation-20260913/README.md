# Residual identity cancellation counterexample

A small exact-integer fixture demonstrates that the localized squared-norm
identity can report an order-one error while both ordinary and localized
residual coefficients are exactly correct. It does not change the residual
assembly, the existing identity metric, its threshold, or any PDE gate.

At a retained mode, D=-100000000, C=100000001 and V=0 give R=1 exactly in
binary64. At a shell mode, D=V=0 and C=7 give R=7. All three components and their
required conjugate partners are checked against an independently constructed
coefficient oracle. The retained L2 identity metric exceeds 0.9; both shell
identity metrics equal zero. The cause is cancellation when separately rounded
squared norms and cross contributions near 1e16 are added to recover an energy
near 1. The coefficients themselves do not undergo that expansion.

This reproduces a mechanism consistent with the earlier clock 1112 observation:
retained/full-band identity failures alongside exact shell identities and a
replayed residual hash. It does not rigorously bound the earlier reduction
error or prove that every archived discrepancy has this cause. Existing failed
checks remain failed and preserved. The full force-representation residual
remains a separate numerical question.

Source 0928b2e integrates the independent Terra regression. The five tests in
`cargo test -p nsbu-solver --test residual` pass. Clippy for that target also
passes with warnings denied. This test-only increment establishes no PDE
trajectory, reconstruction qualification or accepted window.
