# P08 bounded window measurement review

This increment reviews supplied numerical measurements. A passing review returns
`ReadyForLineageReview`; it never accepts a PDE window or claims an enclosure.
**P08 remains incomplete and accepted concentrating windows remain zero.**
The complete experiment still needs the benchmark's frozen observable inventory,
same-problem and current-grid identities, actual accepted-state provenance, and
complete checkpoint/refinement lineage. [summary.json](summary.json) records the
exact source, configurations, tests and remaining limitations.

## Numerical policy

Every required time/observable pair has a tracking-error test and eleven separately
reported channels: space, time, method, force resolution, force precision,
arithmetic, reference precision, transfer, sampling, reconstruction and quadrature.
The immutable allocation ledger conservatively sums binary64 budgets, including
rounding residuals, and refuses over-allocation. A budget is bookkeeping; it does
not establish numerical accuracy. Full-field, unaligned observations are required.
Common-band or aligned agreement cannot advance the review.

Space, time, sampling, reconstruction and quadrature require refinement evidence.
A two-level sensitivity result cannot substitute for that sequence. A plateau,
including two observed zero differences, is inconclusive unless a separately
identified subordinate-floor analysis is supplied. The experiment layer must
verify that analysis and its applicability; a nonzero digest alone is not proof.
The numerical rule requires observed changes within that bound, which must occupy
strictly less than the declared fraction of its channel budget.

Three strictly nested exact tick sets share the same endpoint and include time
zero. The finest set contains explicit off-stage probes with three nested accepted
history geometries. Quarter-stage, collocation-only, mismatched-clock, future-history
and endpoint-extension substitutions are refused. These probes identify sampled
points; they do not establish complete interval coverage or rigorous maxima.

The streaming collector preflights record counts, borrows caller-reserved manifests
and allocates no storage. Malformed observations spend an attempt while preserving
the next required position and previous findings. All numerical channels remain
visible in a valid failed record; failures cannot be cleared by later successes.
Missing observations, exhaustion and numerical rejection have distinct statuses.

## Verification and review

Negative controls cover missing channels, incomplete time/observable inventories,
common-band and aligned comparisons, false zero plateaus, absent floor analysis,
over-allocated and underflowed budgets, endpoint extension, and stage-only probes.
The full workspace test, coverage and mutation reports include the earlier actual
from-rest smooth and concentrating diagnostics; these policy tests do not turn
those unresolved concentrating trajectories into qualified results.

SOLID review separates exact sampling geometry, refinement rules, budget allocation,
observation evaluation and streaming scheduling. The collector cannot mutate a
trajectory, access an analytical reference or import an accepted-window label.
Shared scalar admission helpers remove duplicated rule logic without combining
independent numerical oracles. No dependency or frozen input changed.

The local quality report measures each requested metric across maintained Rust
source and tests. CI now checks per-function cyclomatic and cognitive totals,
including nested closures, matching the local metric interpretation. Four long
scientific studies run in full tests and coverage and are skipped only for mutation
reruns. No production code is excluded and no equivalent-mutant exemption is used.
Hosted verification of this increment remains a separate pending gate.


All local gates passed: 195 tests/probes; 9,733/9,733 executable lines,
810/810 instrumented branches and 792/792 functions across 142 Rust files.
The full 2,175-mutant run caught 2,023 and found 152 unviable, with zero survivors
and timeouts. Maximum CC21, cognitive15, Halstead74.4231, physical-file336 and
CRAP21 satisfy the requested limits. Clone detection and strict lint/review find
zero redundant/dead-code findings and no dynamic type escapes. Fresh packaging,
all 41 bootstrap tests, repository checks and the original mathematical checks
pass. Frozen inputs remain byte-for-byte unchanged.
