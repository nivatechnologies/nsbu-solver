# navier-runtime: construction and convergence entry point

Revision 0.8 · 8 September 2026 · Adopted engineering baseline revision 0.7

The runtime and manufactured experiment are specified in [COMPLETE_DESIGN.md](COMPLETE_DESIGN.md), which remains unchanged at revision 0.7. The source inventory is [CONSTRUCTION_LEDGER.md](CONSTRUCTION_LEDGER.md); the release priorities and retained future admission requirements are in [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md).

The original literal target is independent integration of one fixed realization of the source force from rest over successively closer pre-singularity intervals. It remains unmet and outside the current release scope. The general status is `FeasibilityUnestablished`. The explicitly defined slow-mesh subdivision strategy is `FeasibilityExcluded` under [source-feasibility-policy.json](source-feasibility-policy.json). Those labels assess different scopes; a resource exclusion for one plan is not a theorem against all full-PDE representations.

The compiler's current role is mathematics verification only. It may check selected identities, parameter inequalities, pointwise fields or finite-stage residuals with stated bounds. It cannot advance the independent PDE frontier. A stress surrogate remains a separate deferred decision and would require its own problem identity.

`similarity-mms-v2` is the active concentrating implementation target. It has fixed continuum fields, a prescribed force, positive viscosity and rest initial data. It has no source pulse cascade or verified all-order force extension. Its mathematical manifest and hash are unchanged. The radius-screen reach is three halvings at `512^3`; no convergence-qualified window currently exists.

Retain full-band comparisons and derivative-sensitive diagnostics, pure prescribed-force evaluation, independent evolving state and reference pathways, same-problem refinements, from-rest lineage, inherited error and invalidation. Empirical sampled evidence must not be presented as an enclosed slab-wide result. Any future reopening of literal reproduction requires actual source admission, complete force/reference accuracy and a new representation-specific feasibility report.

The [assessment](ADVERSARIAL_REVIEW.md) and [source report](SOURCE_FEASIBILITY.md) correct the amplitude inequality, full support width, auxiliary-time interpretation and heuristic carrier estimate. The [script](navier-runtime-verification.py) and [results](navier-runtime-verification-results.json) contain executed algebra and arithmetic only. No source-admissible profiles, full force artifact, Rust solver or PDE trajectory have been produced here.
