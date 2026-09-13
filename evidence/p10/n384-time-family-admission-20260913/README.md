# N384 24/48/96 time-family admission plan

This is a source-bound read-only plan. No trajectory was run, no active profile was changed,
and no numerical or PDE-window acceptance is claimed. Here `24/48/96` means accepted macro
attempt counts. The nested schedules are h128/h256, h64/h128, and h32/h64, each switching at
clock 2048 and ending exactly at 4096.

The source computes one full step and two chained half steps before it examines local
tolerances. Only the two-half-step field is written to private candidate storage. Tolerances
form the denominators of the velocity and curl discrepancy ratios; `ratio <= 1` only issues a
commit token. The advective value is likewise checked after an RHS output is computed. The
committed payload changes only when the validated candidate is swapped into it. Consequently,
an already-passing attempt selects the same precomputed candidate under a looser local gate.
This source-level implication does not establish cross-binary reproducibility and does not
permit archived metadata to be relabelled.

The current h64 startup/onset maximum ratios are 0.1264035545 in L2 and 0.9636019681 in H1.
An h-fifth screening extrapolation gives 4.0449 and 30.8353 for h128, and 129.44 and 986.73
for h256 from startup. These are deliberately labelled heuristic: the design explicitly says
the raw full/two-half discrepancy is not a certified bound and current-regime `OrderEvidence`
is required before using a Richardson order. Late h128 maxima are only 0.0011706 and
0.0077619, whose corresponding h256 screen is 0.03746 and 0.24838. This supports placing h256
only after clock 2048, but it does not guarantee that either coarse segment will pass.

The only already-established common admission is absolute `[1e-5,1e-4]`, relative
`[1e-5,1e-5]`, and advective guard 3.3; it carries a serious coarse-start rejection risk. The
JSON records a separate refusal-preserving pilot candidate with all local tolerance components
scaled by 64 and the duration-scaled guard 6.6. That candidate leaves the global scientific
targets unchanged, but it is not approved as a production trajectory policy because the
read-only evidence cannot validate its actual h128/h256 behavior.

The design requires three time settings, raw comparisons, predeclared budgets, and honest
order evidence. It does not literally require identical local-tolerance words among temporal
branches. The current family API nevertheless supplies one shared tolerance/guard policy, and
the snapshot `TIME_DIAGNOSTIC` adapter compares local tolerance bits exactly. Therefore the
existing completed 48-attempt state cannot currently be paired with new branches carrying the
conditional policy. Reuse needs either literal common metadata on all three runs or a reviewed
adapter/policy extension that binds every old attempt and state hash and proves componentwise
tighter-gate dominance without rewriting the old manifest.
