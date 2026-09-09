# Adversarial review and completion of navier-runtime revision 0.8

Revision 0.8 · 8 September 2026

Act as an independent adversarial reviewer and numerical-methods designer. Assess the supplied design; do not assume its corrections, equations, feasibility judgments, or evidence labels are right. Resolve questionable points with primary sources and reproducible calculations where possible. Continue from sound work without silently changing the research objective.

## Inputs and completeness

The four governing inputs are `ADVERSARIAL_REVIEW.md`, `COMPLETE_DESIGN.md`, `CONSTRUCTION_LEDGER.md`, and `IMPLEMENTATION_PLAN.md`. Also inspect `navier-runtime-verification.py` and its actual `navier-runtime-verification-results.json`. The two shorter design files are entry points, not additional competing specifications. The optional Niva adapter is excluded and is unnecessary for this review.

Use either the separate files in `navier-runtime-review-bundle.zip` or the complete embedded sections in `navier-runtime-review-packet.md`. When files are available, check their SHA-256 values against `navier-runtime-review-manifest.json` and rerun the script when its dependencies are available. Report missing or truncated input and its effect on the review; do not claim to have inspected text you did not receive. This prompt ends with `END_NAVIER_RUNTIME_REVIEW_PROMPT_REV_08`; the packet has its own final completion marker.

Primary sources:

- NS manuscript: https://cdn.openai.com/pdf/32d9f210-8b73-45e0-91bc-82a30aef8a9a/navier-stokes.pdf
- Formalization: https://github.com/openai/NavierStokesAndEuler
- Separate Euler visualization: https://github.com/pmocz/euler-blowup-viz
- Hochbruck–Ostermann manuscript: https://publikationen.bibliothek.kit.edu/1000042061/3153602

The existing source-PDF byte checksum and exact repository-commit checks are incomplete. Do not confuse local design-file hashes with verified source provenance. Attribute repository-specific claims until you inspect their actual supporting files. No Rust compilation, PDE integration, source-instance admissibility computation, or formal build is established by the supplied verification results.

## Objective and fixed boundaries

The original research target is independent, convergence-checked integration of one fixed realization of the paper's prescribed-force, incompressible three-dimensional Navier–Stokes problem from rest over successively closer pre-singularity intervals. It is outside the current release scope and remains unmet. The active release targets are the adopted revision-0.7 runtime and `similarity-mms-v2`. Review the separate `FeasibilityExcluded` explicit-mesh policy and `FeasibilityUnestablished` broader target without collapsing them. The optional compiler is mathematics verification only; a surrogate is a separate deferred model decision.

The standalone runtime, construction compiler, and explicit manufactured benchmark are useful deliverables with distinct evidence. `similarity-mms-v2` has accessible parameters, simplified profiles, and a fixed continuum residual force. It does not implement the source cascade or claim a smooth force through the target time. Do not let successful manufactured tracking stand in for the original reproduction target, or equate finite-interval convergence with a singularity proof.

The public Rust library, CLI, tests, examples, documentation, and dependencies must work without Niva. Its optional adapter is a separate downstream consumer. Visualization follows solver validation. Do not substitute Euler, a reduced ODE, hyperviscosity, an unresolved stress closure, or a reference-driven trajectory for the stated PDE.

## Revision 0.8 receipt and targeted questions

`COMPLETE_DESIGN.md` and `similarity-mms-v2.json` intentionally remain at revision 0.7 and are hash-frozen. The surrounding review materials are revision 0.8. Do not change those baseline bytes merely to align version numbers. Any proposed engineering repair must identify the concrete defect and create an explicit proposed amendment. Also inspect `source-feasibility-policy.json`; its 1/4 chart segment and 1,024 stored-interval cap are declared policy assumptions, not measured hardware capacity or an admitted active-support inventory.

Read every supplied component before concluding. Explicitly list the files actually received, including `CONSTRUCTION_LEDGER.md`, `IMPLEMENTATION_PLAN.md` and `navier-runtime-verification-results.json`; they were omitted from an earlier reviewer upload. The single packet embeds their complete contents and completion markers. Missing inputs are missing evidence, not assumed approval.

Also read `SOURCE_FEASIBILITY.md` and `similarity-mms-v2.json`. Assess the localization argument against Proposition 10.1 and Lemmas 10.2–10.3. Check the distinction between a free finite-window cutoff and global profile dependencies or a cutoff changed between windows. Verify the time/box scaling rather than calling translation a change of duration.

Review all four extracted quantities: actual `q_star` versus a merely sufficient screen; slow mesh, bounded point overlap and label/inverse workload; physical phase and auxiliary map scales; actual background/correction cutoff sequences and initialization activity. Verify the relaxed `ell=11217670` calculation without treating it as an admitted or universally necessary band. Upper count bounds are not universal computational lower bounds. A failed sufficient inequality does not disprove feasibility under sharper estimates.

Check the amplitude-inequality correction with all omitted logarithmic/profile/weight factors; distinguish mesh spacing from full support extent, and auxiliary traversal from the intersection defining a physical pulse. Check Lemma 7.7 before treating `k/ell^6` as a finite necessary admission condition. Independently recompute the heuristic crossing and distinguish it from `q_star`. Inspect the source's disjoint auxiliary supports; neither generic partition cancellation nor support spacing alone settles summed-field complexity. Require a stated norm, tolerance and representation class for any stronger exclusion or feasibility claim. Keep unverified pinned Lean file contents attributed to the reviewer.

For the runtime, check the retained v2 identity and rational parameters, three-dimensional diagnostic masks/coverage, exact tick clock and conversion/capacity limits, preallocated swap commit, full HO row identities, binary64 coefficient junctions, and the specified 80/120-digit small-grid arithmetic comparator. That comparator does not by itself qualify a large-grid window. The existing script is not a PDE solver.

## Phase 1: adversarial review

For each finding state severity, exact affected requirement, evidence or counterexample, scientific consequence, proposed repair, and repair/verification status. Separate correctness errors, incomplete design, untested implementation, and limited source access. Review the following seven areas.

### A. Source scope, scaling, and feasibility

Check the governing theorem/problem, force regularity, initial data, domain, viscosity mapping, support, and target time. Recheck Lemma 4.8 and its parameter dependencies; distinguish relaxed boundary calculations from an admitted instance. Verify the ceiling-carrier threshold and Reynolds comparability, not just their quoted decimals. Assess what the annulus ratio proves for a specified representation and error norm. Separate tiny-time representation from subtraction, phase accuracy, cancellation, and full-PDE resolution. Do not turn an unsupported feasibility promise into an unsupported universal impossibility claim.

### B. Explicit manufactured field and construction compiler

Derive the `similarity-mms-v2` root bracket, admissible branch, streamfunction and Cartesian field, pressure, cutoff, startup from rest, and continuum force. Check axis limits, required derivative orders, jet coefficient conventions, implicit root differentiation, cutoff underflow, and force cancellation. Identify precisely which source identities it does and does not satisfy. Review ledger objects S00–S16 and compiler stages C0–C6: parameters, exterior/axis profiles, moments, cone margins, background recursion, pulse/phase dynamics, mean corrections, residual cycle, localization, active terms, and tails. Generic callbacks cannot hide missing central mathematics.

### C. Force, precision, and independence

Check the fixed mathematical problem versus its approximate artifact identity. Verify that forcing has no evolving-state input and the integrator has no reference-state input. Assess continuous versus sampled-force errors, aliases, derivative and phase bounds, scaled time, and arithmetic limits. Force rounding is not an integrator-arithmetic study. A residual constructed from an independently defined continuum field may prescribe a manufactured force; a residual cancelling the live numerical trajectory is a different experiment. Check immutable coverage, appended construction prefixes, and inherited error.

### D. Spatial and temporal algorithms

Derive Fourier normalization, half-spectrum conjugacy, mean and Nyquist handling, padding/cropping, projection, pressure sign/gauge, and exact retained quadratic convolution. Independently derive all CM ETDRK4 weights, large-negative branches, remainders, zero limits, stage times, and constant-source identity. Verify the complete HO five-stage tableau against its primary source and test nonautonomous stiff cases. Assess stiff order reduction and the scope of empirical timestep indicators; a frozen pilot Richardson divisor is insufficient without current-regime evidence. Review bounded attempt costs, pure nonmonotone force callbacks, and transactional acceptance.

### E. Successive intervals and false-success detection

Check the endpoint rule, from-rest branches, checkpoint lineage, full-band comparisons, local derivative norms, force/reference/arithmetic/transfer studies, declared tolerances, and cumulative error. Common-band agreement and sampled residuals alone cannot certify a window. Derive the continuous-defect estimate and specify what requires an actual enclosure. Physical instability does not waive failed tracking tolerances. Require diagnostic responses or honest relabelling for high-mode contamination, unresolved force aliases, lower force precision, changed force across extensions, reference-seeded starts, and coarse-to-fine history loss. Pulse-removal tests apply only after active source pulses exist.

### F. Representation, resources, and Rust contracts

Audit allocation lifetimes, all FFT/provider/diagnostic scratch, operation counts, size overflow, public types, ownership, candidate validity, commit tokens, epochs, rejection rollback, restart completeness, force-provider purity, and trajectory isolation. Distinguish bounded operation counts from hard wall-clock promises and profile-scoped determinism from cross-hardware bitwise equivalence. A smaller-scale backend must supply transformed PDE operators, global pressure coupling, nonlinear interactions, boundary/axis treatment, transfers, and overlap evidence; naming adaptivity does not specify them.

### G. Evidence and readiness

Rerun and challenge the verification script where feasible. Check whether each assertion actually tests its intended claim and whether its output supports the accompanying prose. Identify untested central assumptions and the next discriminating calculation. A clean algebra check does not constitute a Rust build or a PDE result. Check public-only build/review boundaries and source provenance. State separately whether the runtime, manufactured evaluator, experiment harness, and literal source construction are ready to implement or still require mathematical decisions.

## Phase 2: repair and complete the design

Produce a consolidated review and any necessary proposed repairs. Preserve the adopted revision-0.7 runtime and case unless an identified defect justifies a separately versioned amendment. Source research is optional and must not block the release path. Resolve central obligations directly where possible; keep exact unresolved quantities and gates visible when evidence does not support a completion claim.

The replacement must include:

1. **Exact problem contract.** Equations, units, initial data, force, domain, source mapping, localization, target time, and immutable construction identity. Give distinct identifiers and permitted result claims for every benchmark and source instance.
2. **Concrete construction recipe.** Equations, dependency order, profile solvers, admissibility inequalities, deterministic parameter choices, active-term enumeration, derivatives, pressure/force, precision policy, and bounds or explicitly labelled estimates. Supply a checked numerical source instance only when supported; an unevaluated choice is not a checked one. Fully specify the manufactured member without implying it meets source conditions.
3. **Independent evolution algorithm.** Spatial and temporal operators, pressure/nonlinearity, actual stage forcing, coefficient evaluation, error indicators, state transfer, stopping reasons, and complete smaller-scale equations when proposed.
4. **Successive-interval experiment.** Endpoint rule, retained history, overlap comparisons, refinement matrix, predeclared tolerances, cumulative error, invalidation/reruns, and admissible evidence labels. Separate sampled empirical evidence from slab-wide enclosures.
5. **Rust architecture.** Public types and meaningful signatures, dependencies, ownership, callback purity, candidate validity, configuration, restart, and standalone CLI flow. Central mathematical work may not disappear behind an unimplemented trait.
6. **Verification and feasibility.** Independent analytic/manufactured fixtures, actual full-PDE comparisons when performed, negative controls, separate precision channels, complete memory/work accounting, and the last-resolved-interval procedure. Label proposed tests, estimates, measurements, and rigorous bounds distinctly.
7. **Implementation plan.** Small ordered tasks with dependencies, outputs, acceptance tests, and scientific exit criteria. Put literal-construction admission and feasibility before literal-reproduction claims. Keep visualization after numerical validation and Niva integration independent.

Include negative controls for removal of an active pulse when applicable, high-mode contamination, force aliasing, reduced force precision, changing force between extensions, reference-state resets, and unsupported refinement/restart history. Specify which diagnostic should respond; not every change must produce a large global effect.

## Required deliverables

Return four Markdown documents, or four clearly separated sections when file creation is unavailable:

- `ADVERSARIAL_REVIEW.md`: ranked findings, evidence, falsification tests, and repair status.
- `COMPLETE_DESIGN.md`: consolidated governing specification, with explicit limits on completeness.
- `CONSTRUCTION_LEDGER.md`: every required source object, equations, representation, choices, admissibility, precision, dependencies, and actual validation status.
- `IMPLEMENTATION_PLAN.md`: ordered work and evidence sufficient to realize each stated experiment.

Include any verification scripts, commands, and actual outputs. Cite primary sources beside supported claims with equation/theorem references. Report inaccessible sources precisely. Keep private adapter details outside the public review.

End with an honest readiness assessment: what is implementation-ready; whether an admissible source instance and any accepted PDE windows exist; what was actually executed; and the precise remaining mathematical/numerical obligations. Do not label the literal reproduction complete because the runtime design or manufactured benchmark is specified. Do not end at generic instructions to implement the paper, add adaptivity, or use enough resolution.

END_NAVIER_RUNTIME_REVIEW_PROMPT_REV_08
