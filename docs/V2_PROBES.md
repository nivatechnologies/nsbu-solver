# Exact-v2 off-stage reconstruction probes

`v2_experiment::probes::ProbeFamily` owns six independent exact-v2
`ReconstructedRun` trajectories.  It streams a fixed manifest of non-stage
physical clocks before each owner's three-node accepted history is overwritten.
The default `V2Family`, runtime CLI and checkpoint formats are unchanged.

`ProbePlan` reuses the admitted settings for the existing six-branch schedule and
constructs a reconstruction-capable plan for every branch.  Before allocation it
checks the exact floor/ceiling three-node Hermite geometry for every branch and
probe clock.  Its joint reservation covers all six owners, six complete
velocity/derivative scratch pairs, comparison work for both channels, report
metadata and a finite attempted-report allowance.

Each `advance` clears the previously published view, charges one complete attempt,
and advances each branch only far enough to obtain its required third node.  A
sample becomes visible only after all eighteen velocity components and eighteen
derivative components are reconstructed and all ten pair comparisons succeed.
A stopped branch or invalid history terminates the family without exposing
partial scratch or permitting a retry.  Every report retains the exact three
accepted clocks and current state clock for all six branches, plus an identity
that binds the underlying exact-v2 family and complete probe manifest.

The focused startup test streams clocks before, between and on accepted macro
endpoints.  It checks all value and derivative comparisons with a separate
full-complex integer-mode reducer and compares every final branch state, history
and work ledger to a separately evolved reconstruction owner.  At the late
clock 95, interpolation weights come from a separate dense 6-by-6 Hermite solve
over independently evolved accepted nodes 64, 80 and 96.  The node derivatives
are separately assembled with the observer's configured `M=24` force-sampling
profile and fresh `V2Force` and `ConservativeWorkspace` evaluations.  That
control is independent of stored observer nodes but reuses production PDE
operators and is not a wholly independent PDE-operator oracle.

This family does not assemble an off-stage residual, compare an analytical
reference, import reconstruction history, qualify force sampling or establish a
continuous-time bound.  Its sampled differences do not accept a concentrating
window.  P09 and P10 remain incomplete.
