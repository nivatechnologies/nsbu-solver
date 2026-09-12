# Caller-supplied review geometry evidence

This artifact binds the fixed 3/5/7 manifests and four off-stage refinements at source commit `b66fc14aedd0d3544196a4817785a6c73409e780`, following the earlier evidence commit `0c7dbecc653542ca872063f5ca09b79caa590397`. The constructor copies fixed caller arrays, requires strict nesting, binds coarse clocks to the actual `FamilyPlan` manifest and fine clocks to the actual `ProbePlan` manifest, recomputes all node triples, and derives both identities from the immutable plan.

The nine focused tests pass. They include a plan-only independent first-window schedule (0/2048/4096), valid geometry, non-nested, duplicate, missing-clock, same-clock wrong-node mapping and foreign-identity controls. The startup identity regression remains unchanged. Clippy and Rustdoc pass. The selected-test coverage is only a dependency-closure result (31.7802% lines, 23.1017% branches), not a whole-maintained CI claim. Review-profile CRAP peaks at 18.1181; `from_manifests` is CC 10, cognitive 4 and CRAP 11.5625.

This increment retains `PartialUnpopulatedDiagnostic`. It publishes no values and makes no readiness, provenance, pressure, coverage, convergence, or PDE-window claim.
