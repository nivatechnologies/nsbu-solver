---
name: nsbu-numerical-experiment
description: Plan or assess NSBU concentrating-PDE experiments and their acceptance evidence. Use for numerical run design, refinement comparisons, or window qualification.
---

# NSBU numerical experiments

From the checkout root, consult `docs/NEXT_CONCENTRATING_WINDOW.md` for unresolved
dependencies and the relevant section of `IMPLEMENTATION_PLAN.md` for exit
criteria. Read frozen `docs/design/COMPLETE_DESIGN.md` when resolving a mathematical
contract. Read only the evidence relevant to the question being tested.

Choose the cheapest experiment that distinguishes the leading explanations for
the current failure. State the hypothesis, decision-changing outputs and matched
comparison variables before launching. Measure representative throughput before
committing to an expensive family; account for contention and the user's deadline.
Reuse valid source-bound runs when their settings actually match.

The first exact-v2 concentrating interval ends at tick 4096 with quantum 2^-20
and T*=1/128. Tick 2048 is a feasibility midpoint. Evolve each comparison branch
independently from rest. Use analytical reference spectra to guide resolution,
never to replace integrated states or to certify PDE convergence.

Freeze candidate observable budgets before confirmation. Keep spatial, temporal,
force, arithmetic, reference and sampling errors distinct; match endpoints and
control other variables when attributing a difference. The active plan requires
three spatial grids, temporal refinement and a higher-order comparison, and all
mandatory physical/regional/pressure/balance and provenance channels. A faster
bit-identical kernel is not improved-arithmetic evidence; a small-grid oracle is
not a current-grid arithmetic study. Consult the active plan for the full gate.

Retain commands, source and harness hashes, input configuration, exact clocks,
resource limits, attempts/commits, exit status and output hashes. Preserve original
reports; place fresh work in `work/` until suitable evidence is archived. Record
partial runs and failures without treating them as completed endpoints. Commit
focused source/evidence changes within existing authorization; publication follows
the user's release scope.

Return the measured result, what it rules out, and the next unresolved decision.
Distinguish execution, refinement agreement and verifier acceptance. Update
`project-status.json` only when demonstrated capability changes; do not infer an
accepted window from an encouraging ratio or a heuristic grid screen.
