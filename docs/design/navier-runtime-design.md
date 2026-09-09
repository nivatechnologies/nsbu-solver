# navier-runtime: standalone Rust solver design

Review/package revision 0.8 · 8 September 2026 · Adopted runtime design revision 0.7

The governing engineering specification is [COMPLETE_DESIGN.md](COMPLETE_DESIGN.md), adopted byte-for-byte at revision 0.7. The [v2 case manifest](similarity-mms-v2.json) is likewise unchanged. [ADVERSARIAL_REVIEW.md](ADVERSARIAL_REVIEW.md) records the revision-0.8 assessment and release decisions; [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md) gives the ordered implementation work.

The public runtime is a standalone three-dimensional incompressible Navier–Stokes library and CLI. Niva integration is an optional downstream adapter outside the public Cargo workspace. No Niva account, code, private data, registry, schema or service is required. The adapter document is excluded from the public review package.

The release targets are the runtime and independent `similarity-mms-v2` convergence experiment. This manufactured case has fixed equations and a prescribed continuum residual force. It does not implement the source pulse cascade or establish a smooth force through the target time. The three-halving reach at `512^3` is a radius screen, not three accepted PDE windows.

Literal-source reproduction is outside the current release scope. The explicit slow-mesh subdivision plan is `FeasibilityExcluded` under [source-feasibility-policy.json](source-feasibility-policy.json); the general literal target remains `FeasibilityUnestablished`. The [source report](SOURCE_FEASIBILITY.md) and [construction ledger](CONSTRUCTION_LEDGER.md) give the source, arithmetic and scope of these distinct statements. Optional compiler work is mathematics verification only. An averaged-stress surrogate is deferred as a separate model decision.

The independence protocol retains from-rest lineage, full-band comparisons, separate force/reference/arithmetic refinement, inherited error and invalidation of unsupported descendant windows. No reference-field reset can advance the independent frontier.

The [script](navier-runtime-verification.py) and [actual results](navier-runtime-verification-results.json) are executed design checks. No Rust compilation, PDE integration, source-instance admission or formal build has been performed. Use the complete [review prompt](navier-runtime-adversarial-review-prompt.md) and [packet](navier-runtime-review-packet.md), or the separate-file ZIP. The [manifest](navier-runtime-review-manifest.json) records component hashes, frozen baselines and completeness checks; these do not certify source-PDF provenance.
