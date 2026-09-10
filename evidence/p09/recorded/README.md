# P09 recorded-step and balance-history components

This increment adds bounded recorded attempts and restartable controller/balance
components. It does not complete P09 or qualify a concentrating PDE window.
The exact tested source, configurations and raw reports are in
[summary.json](summary.json).

The fixed controller retains method, exact endpoint/step, tolerances, attempt
allowance, accepted count and terminal rejection/refusal. Every attempted outcome
is recorded in pre-reserved storage, including available local error indicators.
Diagnostics measure the private candidate through a read-only observer. A
single-use prepared transaction excludes concurrent state/candidate mutation;
all fallible diagnostics and history checks precede the infallible physical swap
and log publication. Failed attempts preserve physical state and accepted balances.

`BalanceHistory` retains compensated energy/enstrophy quadrature, previous nodes,
initial quantities and a pending Simpson midpoint. Incomplete pairs remain
explicitly incomplete. Each pair requires exact equal half-spans. Finite inputs
whose accumulated defect overflows are refused before publishing the new history.

Actual CM and HO tests cover accepted runs, local rejection, source failure,
diagnostic failure, malformed measurements, later failure after an earlier commit,
wrong clock/count/method, exact resource caps and terminal scheduling. Private
corruption fixtures additionally test stale holders and inconsistent proposal
metadata. A separate allocation-instrumented executable performs 20 complete
recorded commits for each method with zero allocations, reallocations or frees
inside the attempt/diagnostic/log loop. The workspace now has four such allocation
probe executables. Physical-image and balance-history tests retain the separate
next-attempt and pending-compensation guarantees of their components.

All 221 tests/probes pass across 168 maintained Rust files. Coverage reaches all
11,612 executable lines, 942 instrumented branches and 926 functions. Maximum
cyclomatic complexity is 21, cognitive complexity 16, function/file Halstead
difficulty 74.4231, physical file length 336 and CRAP 21. Strict Clippy, clone
analysis and scoped dead-code/public-API review find no findings or dynamic type
escapes. The complete mutation accounting is recorded in summary.json; invalid
mutants are reported separately and never counted as kills. Four long scientific
studies run in full coverage and are skipped only during mutation reruns. No
production exclusion or equivalent-mutant exemption is used.

SOLID review keeps scheduling, observation, balance arithmetic, storage and
transaction ownership separate. The coordinator depends on narrow RHS and
read-only observer contracts. Shared finite-source and resource-plan fixtures
remove duplicated setup without merging independent numerical oracles. Existing
storage allocation failures use the common checked reservation helper.

Fresh public packaging, repository checks, 41 bootstrap tests and the original
mathematical verification pass. Dependencies, LICENSE and frozen reviewed inputs
are unchanged. Hosted checks of this increment are pending.

Complete coherent checkpoints still require authenticated artifacts, actual
provider work/state, reconstruction history and validated ancestry/error reports.
The separate draft adds raw accepted samples and history replay; it is not part
of this source snapshot. Independently supplied state/history objects are not
proven coherent merely because their clocks and counts agree. Same-problem
refinement/transfer studies, the full window verifier, concentrating numerical
qualification and numerical CLI remain incomplete. Accepted PDE windows: zero.
