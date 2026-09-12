# Fixed-M96 spatial midpoint ladder: user hardware pause

Three independent from-rest cached-force runs used the same frozen exact-v2 case and source `def4730b08025fdd06e7a8a0d78116aea24b6e2c`: N24, N32 and N48, M96, 32 workers, Cox–Matthews step 16. All reached clock 2048 with exactly 128 attempts and 128 commits and emitted validated coefficient snapshots. A subsequent user-requested hardware pause terminated every run before endpoint. This is a user interruption, not a numerical failure; no clock-4096 result exists.

At clock 2048, full-band N24→N32 L2/H1 differences are 0.08569082/8.79870862, and N32→N48 differences are 0.09067440/13.02002519. The H1 differences divided by the independently evolved finer state's H1 norm are 0.19351 and 0.27526. Newly resolved modes account for nearly all H1 in each pair; common-band H1 is 0.08431 and 0.06457. The second adjacent difference and finer-normalized ratio increase, so this midpoint ladder does not show spatial convergence.

The evolved newly resolved H1 magnitudes closely follow the separately measured M384 analytical-reference shells N24→N32 (8.81999) and N32→N48 (13.12833). This explains why adjacent differences need not decrease across unequal spectral shells, but it does not turn either result into an error bound or qualification. Analytical-reference content remains material above N48.

The fresh N24 midpoint is byte-identical to the earlier independent def4730 N24 midpoint. Each run retains its own actual metadata, harness, command, admission record and stdout; no profile metadata is substituted across runs. Complete coefficient snapshots and comparison output are retained. The empty post-interruption process audit confirms no matching owned process remained.

These are spatial/force diagnostics only. They establish no endpoint, convergence, force sufficiency, continuum tail, mandatory channel inventory or accepted PDE window. Accepted windows remain zero.
