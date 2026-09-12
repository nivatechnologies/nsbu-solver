# Caller-supplied review geometry evidence

This artifact binds the fixed 3/5/7 manifests and four off-stage refinements at source commit `22e928992effdb64b4ed71723b68df6395b40278`, based on `b1efef8872a21a0d51ec888b9ce3ab5e20a8af74`. The constructor copies fixed caller arrays, requires strict nesting, binds coarse clocks to the actual `FamilyPlan` manifest and fine clocks to the actual `ProbePlan` manifest, recomputes all node triples, and derives both identities from the immutable plan.

The eight focused tests pass. They include valid geometry plus non-nested, duplicate, missing-clock, wrong-node mapping and foreign-identity controls. The startup identity regression remains unchanged. Clippy and Rustdoc pass. The selected-test coverage is only a dependency-closure result (31.7802% lines, 23.1017% branches), not a whole-maintained CI claim. Review-profile CRAP peaks at 18.1181; `from_manifests` is CC 10, cognitive 4 and CRAP 11.5625.

This increment retains `PartialUnpopulatedDiagnostic`. It publishes no values and makes no readiness, provenance, pressure, coverage, convergence, or PDE-window claim.
