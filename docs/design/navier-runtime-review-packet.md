# navier-runtime: public adversarial review packet

Revision 0.8 · 8 September 2026 · Adopted runtime baseline revision 0.7

This packet embeds the complete review prompt, four governing documents, verification script and actual results, two superseding entry points, source-feasibility assessment, exact v2 case manifest and scoped feasibility policy. The optional private adapter and historical drafts are excluded.

Start with Input 1. Use the separate-file ZIP for machine checks, or read the embedded inputs in order. Component BEGIN/END markers and the final packet marker detect local truncation; they do not guarantee that a receiving model has processed every part.

The original source-construction reproduction remains unmet. It is outside the current release scope. The explicit-mesh policy is excluded; the general representation question remains unestablished. The explicit manufactured benchmark is a separate prescribed-force problem. No Rust build, PDE trajectory, source-admissibility computation, or formal build is demonstrated by these design checks.

COMPLETE_DESIGN.md and similarity-mms-v2.json are intentionally byte-identical to the adopted revision-0.7 baseline. Their older revision tags are correct; the assessment, roadmap, policy and expanded checks are revision 0.8.

The verification script requires Python 3 and SymPy. After extracting the ZIP, run `python3 navier-runtime-verification.py --output rerun-results.json`. The included results are the recorded execution, not a claim that packaging reran the checks.

## Component identities

These SHA-256 values identify the supplied local components. The source PDF checksum is still unverified.

- `navier-runtime-adversarial-review-prompt.md`: 16058 bytes; SHA-256 `fc8de3d5e16ff23f36ba94fcb96b92f2c768cc798a3b2116480e1e6fcd1d359a`.
- `ADVERSARIAL_REVIEW.md`: 10427 bytes; SHA-256 `5a4e87fdfdf9035f35d58eb6f2b69053dcfce69f0963f77086b16463e239369d`.
- `COMPLETE_DESIGN.md`: 49791 bytes; SHA-256 `fabca082cf73fee5f64c7f67800308b5115936bbec04838100a0d2ee425b81d9`.
- `CONSTRUCTION_LEDGER.md`: 23085 bytes; SHA-256 `2a0ca934c8ec9016fdd24235dac7097ab0658213827a0952bce1705a89ed44db`.
- `IMPLEMENTATION_PLAN.md`: 19129 bytes; SHA-256 `c650a2db498112d827021430ab1438ce8ff74c8cb82d28942892e1c5e143f0a3`.
- `navier-runtime-verification.py`: 23525 bytes; SHA-256 `5824a71230c5c01766bce352a73bbfe39b9943ea400058495bec845d8209493d`.
- `navier-runtime-verification-results.json`: 23022 bytes; SHA-256 `92622218cc19c32d54cbf05765901a3e5e3a9d5c58cf4b5536bec47266d170a3`.
- `navier-runtime-design.md`: 2591 bytes; SHA-256 `35abc3b87785d6583231dead5600bedfe83906a0d4f2f41a20487052d3951af9`.
- `navier-runtime-construction-design.md`: 2679 bytes; SHA-256 `94be219ae1c500a9918223843d40201c465e3f90be711746a1dd7aed11ca6221`.
- `SOURCE_FEASIBILITY.md`: 23400 bytes; SHA-256 `a7d245d52be45d8b6cd358adf9104c447d9ada5f0c7e61f2102116b18e63ec00`.
- `similarity-mms-v2.json`: 4940 bytes; SHA-256 `e1236f7b3c51537acd17381402ca420ba7872a7b9dbc64b2f0d9d5108a468f7e`.
- `source-feasibility-policy.json`: 4509 bytes; SHA-256 `b0c9440b7603f67f0b3405bc00cd249636fd060a82188968129e3ea47a4a2e23`.

---

# Input 1: navier-runtime-adversarial-review-prompt.md

<!-- BEGIN navier-runtime-adversarial-review-prompt.md -->

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

<!-- END navier-runtime-adversarial-review-prompt.md -->

---

# Input 2: ADVERSARIAL_REVIEW.md

<!-- BEGIN ADVERSARIAL_REVIEW.md -->

# navier-runtime: revision 0.7 review disposition

Revision 0.8 · 8 September 2026 · Runtime baseline adopted at revision 0.7

## Decision

Adopt [COMPLETE_DESIGN.md](COMPLETE_DESIGN.md) revision 0.7 and [similarity-mms-v2.json](similarity-mms-v2.json) unchanged. Their complete bytes are checked against the archived baseline. This revision changes the source assessment, release priorities, evidence ledger, verification output and review packaging. It does not change the benchmark's mathematical identity or the runtime's numerical contract.

The current release targets the standalone Rust runtime and independent `similarity-mms-v2` experiment. Literal-source PDE reproduction is excluded from this release. The compiler is optional mathematics verification. The averaged-stress surrogate is a separate deferred decision.

Record `FeasibilityExcluded` for the explicit slow-mesh subdivision policy, and `FeasibilityUnestablished` for the literal target across unspecified full-PDE representations. This implements the practical prioritization while rejecting an unsupported universal exclusion. No viable alternative representation has been established here.

## 1. Review receipt and ranked findings

The supplied revision-0.7 review was read in full. It reports receiving the runtime design, source report and script, but not the construction ledger, v2 manifest or results JSON. The rebuilt public bundle includes all of these. Claims in the review were assessed as evidence rather than treated as independent source verification.

| Severity | Finding | Evidence and consequence | Disposition |
|---|---|---|---|
| High | Reversed pulse-amplitude inequality | At `q=Q`, the pure factor is `sqrt(epsilon)<=1/2` for `epsilon<=1/4`. The reported uniform half-background lower bound does not follow; logarithmic/profile/support factors also remain. | Corrected. No small-amplitude omission or universal physical-feature lower bound is inferred. |
| High | Universal representation exclusion lacks its required hypotheses | Exact mesh spacing specifies the construction's partition; it is not by itself a norm-, tolerance- and representation-specific lower bound for every summed-field evolution method. | Exclude the declared explicit-mesh plan; retain the broader unresolved gate and remove literal reproduction from the release. |
| Medium | Slow support width and auxiliary duration were conflated | Full slow support extent is at most `2ell^-6`; `Q^(1+h)L_s` traverses the auxiliary rectangle. Actual pulse support intersects all cutoffs. | Source wording and numeric result names repaired. |
| Medium | Heuristic carrier threshold was promoted beyond its evidence | Lemma 7.7 uses wave classes, phase-normal factors and coefficient bounds, not a bare finite `k>ell^6` admission test. The heuristic crossing has `k≈2.66e43`, not `1e50`. | Actual ratio and predecessor checked; classification remains heuristic. |
| Medium | Provenance promotion would overclaim local verification | Reviewer reports reading the pinned Lean file; this task's file retrieval failed. PDF mesh was inspected directly. | Keep precise reviewer-reported provenance; no formal build claim. |
| Low | Three halvings could be mistaken for converged runs | `512^3` passes the radius screen through `k=3`; comparisons, forcing and other channels can still fail. | Retain as a radius-screen reach only; zero accepted PDE windows. |
| Medium | A surrogate was called the only accessible route and never a full PDE | A prescribed stress can define a different forced-NS PDE; it cannot reproduce the original pulse field merely by matching an auxiliary covariance. | Separate deferred model decision; no exclusivity claim. |

## 2. What the source check establishes

The primary PDF's Section 3.3, Section 6.2, Proposition 7.5 and Lemma 7.7 were checked in text and rendered equations. Full source locations, formulas and arithmetic are in [SOURCE_FEASIBILITY.md](SOURCE_FEASIBILITY.md). This is a targeted source assessment, not a proof audit. [S1]

The amplitude calculation concerns the pure `q^(h/2)` factor, not an absolute pulse bound. At the relaxed quarter band it is `0.4999979261`; at the passing sufficient screen it is `6.3152104e-29`. The source's omitted factors prevent treating either value as the pointwise amplitude of the complete physical field. Derivatives, relative errors, nonlinear stress and force residuals remain mandatory. [S1, p. 11]

The mesh is `ell^-6`; full support extent is at most twice that. Overlapping slow labels have disjoint auxiliary supports after pullback, so an ordinary partition-cancellation example does not settle the physical pulses either. A universal exclusion needs a lower bound on the actual retained field in the declared norm, as well as a precise approximation class and resource model. The review does not supply that result. [S1, pp. 11, 64–66]

The explicit policy does have a straightforward count. Resolve every slow-mesh interval across chart segment length `1/4`, with a declared maximum of 1,024 stored intervals per axis. The exact required counts are about `7.3718e29` and `4.9815e41` at the two relaxed bands. This strategy is excluded by that policy. The segment, cap and assumptions are explicit in [source-feasibility-policy.json](source-feasibility-policy.json); none is presented as a measured hardware limit or as an admitted active pulse region.

The auxiliary traversal and slow support durations can differ because both constrain a label. At the relaxed quarter screen, the proxy `epsilon S` is about `3.5849e9` chart times, while the slow full extent is at most `6.7826e-31`. At the passing screen the corresponding values are about `5.0186e-43` and `1.0037e-42`. Unknown auxiliary constants remain. The word “pulse interval” no longer obscures that distinction. [S1, (6.11)–(6.12)]

The pure carrier/mesh comparison first crosses in its monotone range at band `17274001`. Its predecessor fails and both integer carriers were checked. This is not `q_star`, a proof of the review's necessary-condition claim, or a finite-band curl-remainder estimate. The latter still requires actual phase-normal and coefficient bounds. [S1, (7.38), Lemma 7.7]

## 3. Adopted engineering baseline

The exact tick clock, consumed acceptance token, preallocated swap commit, full-band comparisons, nonmonotone HO stage times, coefficient branches, independent force/reference pathways and separate error channels remain as specified in revision 0.7. This review does not alter or implement them.

The v2 case keeps `T_star=1/128`, `R_in=3/10`, `R_out=21/50`, the fixed continuum force and the same manifest hash. The `512^3` three-halving statement is a twelve-cells-per-peak-radius screen, not convergence evidence. A `128^3` diagnostic pilot cannot qualify the first endpoint under that screen. Smaller pilot grids remain useful for operator, transaction, force and underresolution tests.

The public Cargo project is independent of Niva. Its optional downstream adapter stays outside the public bundle. Visualization remains after numerical validation. No averaged-stress or reference-feedback substitution is introduced into the manufactured case.

## 4. Surrogate decision

Keep the proposed averaged-stress model deferred. Before implementing it, specify the full physical stress tensor and its divergence (including sign, cylindrical geometry and localization), all prescribed inputs and a distinct immutable problem identity. Proposition 7.5 averages independent auxiliary and angular coordinates before the physical map; identifying that object with physical coarse-grained stress needs additional justification. [S1, pp. 11, 82]

A direct numerical solution of such a separately specified forced-NS equation could carry a full-PDE label for that equation. It could not claim reproduction of the original oscillatory velocity, independence of its pulse mechanism, or source force regularity merely because the mean target agrees. No argument here establishes that it is the only possible accessible model.

## 5. Executed evidence and packaging

The expanded [verification script](navier-runtime-verification.py) was executed successfully. It retains the earlier symbolic, coefficient, geometry, exact-clock and memory checks and adds mesh/support ratios, the pure amplitude factor, explicit subdivision counts, auxiliary-time comparisons and the carrier/mesh crossing. The [recorded JSON](navier-runtime-verification-results.json) states the scope of every new calculation.

Packaging retains the complete input receipt, component BEGIN/END and final markers, all seven replacement requirements, per-component hashes, local-link and fence checks, and ZIP membership/CRC/content verification. Revision 0.8 governs the review materials; the adopted engineering design and case remain revision 0.7. Their revision numbers are not silently rewritten. The review bundle includes the ledger, plan, case, results and scoped policy.

The source PDF byte checksum and pinned repository contents are not verified. `NavierStokes/SlotColoring.lean` at commit `8937a8f4cbc7abaab5e9e97d1cc7f5d2319d9538` remains reviewer-reported; its mesh description agrees with the independently read PDF. This review does not promote the Lean comparator provenance in the frozen design. No formal build occurred.

## 6. Readiness

The runtime and manufactured benchmark designs are adopted for implementation. The next implementation packages are the scalar/jet and small-grid arithmetic reference, followed by the standalone Rust workspace and tested Fourier/ETD primitives. Source profile extraction, a literal-source backend, and a surrogate do not block that path.

No Rust solver, scalar/jet implementation, PDE trajectory, admitted source instance or accepted window has been produced by these design checks. An independently evolved, convergence-checked experiment remains work to execute, not a result inferred from the manuscript or manufactured formulas.

## Primary reference

**S1:** OpenAI, [Finite Time Blowup for Navier–Stokes](https://cdn.openai.com/pdf/32d9f210-8b73-45e0-91bc-82a30aef8a9a/navier-stokes.pdf), printed pp. 11, 64–66, 82, 85–86 for the newly assessed claims. Previous targeted extraction remains in the source report. The exact cited Lean module contents were unavailable here; a repository landing page is not evidence of that file or a successful build.

<!-- END ADVERSARIAL_REVIEW.md -->

---

# Input 3: COMPLETE_DESIGN.md

<!-- BEGIN COMPLETE_DESIGN.md -->

# navier-runtime: standalone solver and concentrating experiment specification

Revision 0.7 · 8 September 2026 · Proposed implementation

This is the consolidated engineering specification for the first standalone runtime and an explicitly defined manufactured concentrating experiment. The filename follows the review's requested deliverables; it does **not** mean that the complete source construction has been compiled or reproduced. The original research target remains independently integrating one fixed realization of the paper's force from rest over successively closer pre-singularity intervals. Its unresolved mathematical inputs and admission gate are in [CONSTRUCTION_LEDGER.md](CONSTRUCTION_LEDGER.md).

This document supersedes the numerical and experiment requirements of revisions 0.4, 0.1, 0.5 and 0.6. The supplied revision-0.6 review is assessed in the accompanying assessment; its proposed changes are not adopted without checks. The ranked reasons are in [ADVERSARIAL_REVIEW.md](ADVERSARIAL_REVIEW.md). No Rust implementation or PDE trajectory is reported here.

## 1. Products, evidence, and dependency boundaries

| Product | Concrete output | Evidence it may support |
|---|---|---|
| Runtime | Rust library, standalone CLI, fixed-viscosity periodic 3D PDE engine, diagnostics and checkpoints | Accuracy and reproducibility of the numerical implementation on the stated cases |
| Construction compiler | Explicit source objects, parameter choices, derivative evaluators, inequality and coverage records | The particular mathematical checks actually executed; source-instance status only after the complete admission gate |
| Concentrating experiments | The fully specified `similarity-mms-v2` below; later, separately identified paper-derived profile members | Independent integration and tracking of these prescribed-force problems on accepted finite windows |
| Original reproduction target | A future admitted `source-instance` and a qualified independent trajectory family | Reproduction of that specified source realization on the reported intervals; never a singularity proof from finite samples alone |

`similarity-mms-v2` uses the paper's similarity-coordinate geometry but deliberately simpler profiles. It is a manufactured solution. It does not implement the annular pulse cascade, the paper's heat exterior, its moment and cone construction, or a force proved smooth through the target time. Its increasing velocity is prescribed by its mathematical field definition; the experiment tests whether an independent PDE integrator tracks that field under the fixed force.

The public distribution requires no Niva code, schema, registry, account, service, private data, or feature flag. A Niva adapter is a separate downstream consumer of the same public API, maintained outside the public Cargo workspace. The adapter is not in the solver's release or experiment critical path. The existing optional adapter specification remains separate. A later viewer reads saved evidence and cannot modify integration state or turn rejected windows into accepted ones.

## 2. Equation, geometry, and mapping contract

The reference model is

```
partial_t u + (u dot grad)u = -grad p + nu Laplacian(u) + f(x,t)
div u = 0,     nu > 0 constant.
```

The domain is `R^3/(Lx Z x Ly Z x Lz Z)`, with the unit cube centered at zero as the default. All three velocity components and all three spatial dependencies are evolved. The default quantities are nondimensional; dimensional input additionally declares reference length, velocity, time, density and viscosity conversion. Pressure is kinematic pressure with zero spatial mean. Mean velocity is retained and obeys `d< u >/dt = < f >`. Default Fourier norms use volume averages; reports distinguish these from integrals.

Initial data may be any accepted divergence-free field for the generic runtime. Both the concentrating benchmark and the source target start from rest. Viscosity, force specification, physical geometry, and initial data are immutable within a scientific trajectory. Changing them creates a different mathematical problem. No closure, hyperviscosity, velocity clipping, reference nudging, or state-dependent feedback force is allowed in this reference model.

### 2.1 Source scaling is explicit data

A manifest stores every coordinate and field mapping, its order of composition, support image, initial-time convention, and resulting target time. Two useful viscosity maps from a viscosity-one solution are:

```
Fixed spatial domain:
    u_nu(x,t) = nu u(x,nu t)
    p_nu(x,t) = nu^2 p(x,nu t)
    f_nu(x,t) = nu^2 f(x,nu t)
    T_star(nu) = T_star(1)/nu

Fixed target time on R^3:
    u_nu(x,t) = sqrt(nu) u(x/sqrt(nu),t)
    p_nu(x,t) = nu p(x/sqrt(nu),t)
    f_nu(x,t) = sqrt(nu) f(x/sqrt(nu),t).
```

The first map is the convention attributed to the Lean comparator by revision 0.5; the algebra is checked here, while that exact file remains unverified. The manuscript uses the second map in (10.22). Its Corollary 10.6 then chooses a support-shrinking factor `lambda >= 1`, sets `t0 = 1-lambda^-2`, and uses

```
u_tilde(x,t) = lambda u_nu(lambda x, lambda^2(t-t0))
p_tilde(x,t) = lambda^2 p_nu(lambda x, lambda^2(t-t0))
f_tilde(x,t) = lambda^3 f_nu(lambda x, lambda^2(t-t0)).
```

The fields are zero before `t0` and their separated compact supports are periodized. This convention preserves `T_star = 1`, rather than imposing `1/nu` on all periodic cases. Support containment must be checked for the actual chosen mapping. These transformations are problem preparation, never changes introduced midway through a trajectory. [S1, (10.22), Corollary 10.6]

The localization freedom and four source feasibility questions are now extracted in [SOURCE_FEASIBILITY.md](SOURCE_FEASIBILITY.md). Physical radial and axial cutoffs, activation and every scaling are explicit source-realization parameters. A small onset box is allowed; whole-profile dependencies, actual phase/mesh scales and later inclusion of the shrinking outer annulus remain part of the source gate. This priority screen precedes any commitment to a literal-source backend.

## 3. Exact manufactured member: `similarity-mms-v2`

This member retains the explicit manufactured formulas and changes the onset scale and localization. It has a new identifier because changing these constants changes the mathematical problem. The previous `similarity-mms-v1` with `T_star=1/64`, radii `1/4,3/8` remains a historical case; its runs cannot be resumed as v2. Every formula below is independent of grid, timestep and integrated velocity. The proposed input artifact is [similarity-mms-v2.json](similarity-mms-v2.json); it is not yet consumed by an implemented CLI.

### 3.1 Parameters and similarity map

The default manifest is

```
nu = 1
box = [1,1,1], centered at zero
h = 1/8, A = 1/2+h, D = 1/2-h
b = 1/4, j0 = 1/32
T_star = 1/128, t_ramp = T_star/4
R_in = 3/10, R_out = 21/50
initial u = 0 at t = 0.
```

For `0 <= t < T_star`, let `tau = T_star-t > 0`, `r^2=x^2+y^2`, and define the positive admissible root

```
q - z^2 q^(2h) = tau,
eta = z q^(-D),      X = r^2/(2q),
L = 1-2h eta^2,      d = 1-eta^2.
```

Then `|eta|<1` and `L>1-2h`. The geometry follows the source's (3.2), (4.1), and Lemma 4.1; the parameter `h=1/8` intentionally does not satisfy its construction hierarchy. [S1]

For the general permitted benchmark range `0<h<1/6`, define

```
a(eta) = eta+j0
M(X,eta) = a(eta) X exp(-X)
U(X,eta) = a(eta)(1-X)exp(-X)
F(X,eta) = b exp(-X)
E(X,eta) = sqrt(2X) F(X,eta)
Pi(X,eta) = -(b^2/2)exp(-2X)
V0(X,eta) = [2 eta X U - 2D eta M - d partial_eta M]/L.
```

These profiles satisfy `U=partial_X M`, `M(0,eta)=M(infinity,eta)=0`, and `partial_X Pi=E^2/(2X)=F^2`. The streamfunction is `q^D M`, yielding `u_z=q^-A U` and `r u_r=V0` before localization. These identities are checked symbolically in the accompanying script. No other source moment identity, cone margin, or stress-free region is assumed.

The radii are exact rational values, not exact binary fractions. At the initial equatorial scale the uncut swirl at `R_in` is 1.7633449% of its radial peak, and at `R_out` it is 0.00979469%. These are profile-edge amplitudes, not percentages of removed energy. At `t=0` the actual field is zero because of the ramp. The cutoff is part of the exact manufactured problem, so a substantial cutoff contribution is not numerical error. Report startup and cutoff-region diagnostics separately from interior concentration. The change sacrifices one halving of the individual-grid radius screen relative to v1.

### 3.2 Cartesian formulation and smooth localization

Avoid divisions by `r` in executable field formulas. Define scalar functions

```
G(x,y,z,t) = (1/2) q^-A a(eta) exp(-X)
B(x,y,z,t) = b q^(-A-1/2) exp(-X)
A_vec = G (-y,x,0)
B_vec = B (-y,x,0).
```

`A_vec` is a vector potential, not the scalar exponent `A`. For an arbitrary smooth scalar `G`,

```
curl(G(-y,x,0)) = (-x G_z, -y G_z, 2G+x G_x+y G_y).
```

The Cartesian formulas extend smoothly through the axis for every positive `tau`. To localize, use the original smooth step

```
rho(s) = exp(-1/s) for s>0, otherwise 0
S(s) = rho(s)/(rho(s)+rho(1-s)) for 0<s<1
S(s) = 0 for s<=0; S(s)=1 for s>=1

c_x = 1-S((x^2+y^2+z^2-R_in^2)/(R_out^2-R_in^2))
c_t = S(t/t_ramp)
c = c_x c_t.
```

Evaluate `S` using its logistic form in the open transition interval, with a branch selected to avoid overflow; outside it, return the exact flat value and zero derivatives. Very small values lost to underflow contribute to the evaluation-error record rather than silently becoming an exact support statement. The mathematical cutoff itself is defined by the equations above, not the implementation's underflow threshold.

The prescribed reference pair and force are

```
u_S = curl(c A_vec) + c B_vec
p_S = c^2 q^(-2A) Pi
f_S = partial_t u_S + (u_S dot grad)u_S - nu Laplacian(u_S) + grad p_S.
```

Subtract the spatial mean of `p_S` for reported pressure; it does not affect `f_S`. The squared cutoff in this manufactured pressure is a deliberate benchmark definition, not a claim to follow the paper's pressure localization.

The cutoff is axisymmetric, so `div(c B_vec)=0`; the curl term is divergence-free identically. Every field is zero near the periodic box boundary. At `t=0`, the flat ramp gives `u_S=0` and `f_S=0`, with smooth startup. At the origin after the ramp, the axial velocity is `j0 tau^-A`, so the reference has a prescribed concentrating singular limit. The force is only specified for `t<T_star`; **no smooth extension through `T_star` is claimed**. Each finite closed interval before that time is a distinct domain of evaluation of this same fixed input.

The Gaussian profiles replace the source's heat tail and admissible leading profile. Reports must say `similarity-mms-v2`, not “paper instance” or “reproduction of Theorem 1.1.” A later profile family satisfying additional paper identities receives a separate identifier and an explicit ledger of those identities.

### 3.3 Root, derivatives, and force algorithm

For `tau>0`, bracket the admissible root with

```
lo = max(tau, |z|^(1/D)),  hi = 4 lo.
```

At `z=0`, return `q=tau` directly. Otherwise the residual is nonpositive at `lo` and positive at `hi` for the benchmark range. On the admissible branch its derivative is at least `1-2h`; monotonicity makes the root unique. Use safeguarded Newton iterations inside this bracket with a fixed configured maximum and an explicit residual/bracket-width termination test. Return `CoordinateUnresolved` on exhaustion. A residual divided by `1-2h` estimates the root error; a certified bound additionally includes rounding and power-evaluation enclosures.

Force evaluation uses truncated multivariate Taylor jets in physical `(x,y,z,t)` coordinates. The coefficients store derivatives divided by the multi-index factorial. Total degree three supplies the force; degree four supplies its spatial derivatives and time-derivative diagnostics. Degree four has 70 coefficients. The vector potential's curl means third spatial derivatives are needed for the viscous force and fourth for its gradient; a second-derivative-only profile API is insufficient.

Compute the scalar root first. Construct the implicit jet of `q` by solving

```
F(q) = q - z^2 q^(2h) - (T_star-t) = 0
```

in the truncated polynomial algebra. Formal Newton iteration uses division by the invertible jet `1-2h z^2 q^(2h-1)`. Starting from an exact constant root, three Newton sweeps determine all coefficients through degree four by order doubling. With a rounded root, include its error and explicitly test the residual jet coefficients. Do not obtain derivatives by differentiating an arbitrary finite scalar iteration history. The low-dimensional implicit jet depends only on `z,t`; reuse it across an `x,y` plane when the cache key is exact.

Apply ordinary jet multiplication, division, powers, exponentials, and the tested cutoff branches to the Cartesian formulas. Extract `u_S`, its time derivative, first and second spatial derivatives, and `grad p_S`; assemble the residual algebraically. There are no finite differences of the PDE grid and no input from the integrated velocity. A tile-based evaluator declares its jet storage and maximum root/jet work before planning a bounded attempt. The scalar root residual, differentiation error, arithmetic, cutoff underflow, assembly cancellation, and final conversion are separate accuracy contributions.

A high-precision independent scalar/reference implementation checks jets at axis points, transition collars, tiny positive `tau`, and ordinary interior points. Verify mixed derivative consistency and divergence, and compare finite differences only as an independent convergence test with a demonstrated cancellation floor. Closed-form pressure avoids quadrature for this member. These are required implementation tests; only the listed symbolic identities have already been executed.

## 4. Fourier representation and spatial operators

### 4.1 Storage and normalization

Use real-to-complex half-spectrum storage from the first release: three component arrays, each of length `H=Nx Ny (Nz/2+1)`, with contiguous last index. Grid sizes are positive multiples of four. Forward Fourier coefficients include division by the physical grid's sample count; inverse transforms are the unnormalized Fourier sum. The wrapper must enforce these conventions independently of the FFT library's normalization.

Retain integer modes strictly inside `|m_i|<N_i/2`, with physical `k_i=2pi m_i/L_i`. Set all Nyquist planes to zero. On the stored self-conjugate planes, enforce the remaining conjugacy constraints. Negative last-axis modes are reconstructed by conjugate symmetry, including their first and second indices. Diagnostics using a half spectrum apply the correct multiplicities in Parseval sums; this is an explicit test target.

Padding copies normalized coefficients without changing their amplitudes. Index mappings, zero/Nyquist behavior, negative-frequency derivatives, and crop/pad round trips have small direct-DFT fixtures. The FFT wrapper records library, version, transform algorithm, planning policy, scratch, and SIMD/reduction choices. Dependency selection and pinning happen at implementation time; this design does not claim a particular current release has been installed or tested.

### 4.2 Evolution and pressure

For `k!=0`, use `P_k v=v-k(k dot v)/|k|^2`; set `P_0=I`. The semidiscrete evolution is

```
omega = curl u
d u_hat_k/dt = -nu |k|^2 u_hat_k + N_hat_k
N_hat = P Fourier(u cross omega + f).
```

For each nonlinear evaluation, pad `u` and its spectral curl to `M_i=3N_i/2`, perform six inverse scalar transforms, multiply `u cross omega` there, evaluate the force at the same physical stage time, transform the three result components, crop, and project. This costs nine scalar 3D transforms before extra forcing or diagnostic work. Three-halves padding resolves the retained quadratic convolution under the strict-band convention; do not additionally discard the upper third of the retained band.

Project stage right-hand sides and candidates. Record the projection correction; unexpectedly large corrections are a failure indicator. Preserve the mean mode. An explicit projected initialization is allowed for generic data only when its modification is reported and becomes part of the initial mathematical input.

For physical pressure, retain the unprojected force. With `R=u cross omega+f` and `pi=p+|u|^2/2`,

```
pi_hat_k = -i k dot R_hat_k / |k|^2 for k!=0
p = pi - |u|^2/2, followed by mean-gauge subtraction.
```

An independent conservative-product implementation checks `Laplacian(p)=div(f)-partial_i partial_j(u_i u_j)`. The pressure diagnostic does not feed a different velocity update.

### 4.3 Force sampling is a separate error channel

Three-halves padding prevents retained quadratic aliases, not arbitrary force aliases. A force may have fine frequencies that alias into low modes even when the sampled tail looks small. Before qualifying an artifact, compare its retained Fourier coefficients under at least two increasing force-evaluation grids and use independent off-grid probes or analytic coefficient/derivative bounds. Refine until the declared force budget passes; bounded providers may instead supply directly validated Fourier coefficients with the same normalization.

The step path uses a planned force-evaluation policy validated for the covered interval. Its accuracy report distinguishes continuous force error from transform, sampling, and conversion error. A tail fraction on the runtime's own grid is only a diagnostic. An unresolved force causes `ForcingUnresolved`; lowering the timestep alone cannot repair it.

## 5. Time integration and transactional advance

### 5.1 Cox–Matthews ETDRK4

For a mode let `L_k=-nu |k|^2`, `z=dt L_k`, `E=exp(z)`, `E2=exp(z/2)`, and

```
phi_j(z) = sum_(m>=0) z^m/(m+j)!
Q_dt = (dt/2) phi_1(z/2)
w1 = phi_1 - 3 phi_2 + 4 phi_3
w2 = 2 phi_2 - 4 phi_3
w3 = -phi_2 + 4 phi_3.
```

All weights use argument `z`. With `N` including the projected force, execute

```
n1 = N(u,t)
a  = E2 u + Q_dt n1
na = N(a,t+dt/2)
b  = E2 u + Q_dt na
nb = N(b,t+dt/2)
c  = E2 a + Q_dt(2 nb-n1)
nc = N(c,t+dt)
u_plus = E u + dt [w1 n1 + w2(na+nb) + w3 nc].
```

At zero `z` the method is classical RK4; pure unforced modal diffusion is exponential. These facts are checks, not a universal fourth-order claim for stiff time-dependent forcing. [S2, discussion of exponential schemes and order conditions]

### 5.2 Stable coefficient evaluation

For `|z|<=1`, use a fixed-degree Taylor polynomial for each `phi_j`, with a proved truncation bound below its allocated coefficient tolerance. A 19-term starting design is inherited from the preceding specification; test the combined weights as well as the individual functions.

For intermediate negative `z`, stable algebraic closed forms are

```
w1 = [exp(z)(z^2-3z+4)-z-4]/z^3
w2 = 2[exp(z)(z-2)+z+2]/z^3
w3 = [exp(z)(4-z)-z^2-3z-4]/z^3.
```

For `z<=-50`, evaluate the non-exponential portions using `r=1/z`:

```
w1 = -r^2(1+4r)
w2 =  2r^2(1+2r)
w3 = -r(1+3r+4r^2).
```

The discarded exponential terms are exactly the exponential numerators above divided by `z^3`. Bound them and include them in the coefficient error budget. Using `r` avoids forming an overflowing `z^3`. Validate the branch junction and values near weight zeros against higher precision. Use a conditioning-aware absolute tolerance near cancellation roots; do not impose a universal relative or four-ULP threshold there.

The identities `w1+2w2+w3=phi1`, zero-argument RK4 weights, pure diffusion, and exact integration of a constant modal source are required fixtures. The accompanying script detects the incorrect `w2,w3` asymptotic expressions in revision 0.5. Its successful high-precision comparisons validate this algebra, not a future floating-point implementation.

### 5.3 Independent temporal comparison

Implement the Hochbruck–Ostermann five-stage method as a second tableau engine, with separate coefficient construction. Let `c=(0,1/2,1/2,1,1/2)` and `phi_j,i=phi_j(c_i z)`. Its explicit stages are

```
Y_i = exp(c_i z)u + dt sum_(j<i) a_ij N(Y_j,t+c_j dt)
u_plus = exp(z)u + dt sum_i b_i N(Y_i,t+c_i dt).

a21 = (1/2) phi_1,2
a31 = (1/2) phi_1,3 - phi_2,3
a32 = phi_2,3
a41 = phi_1,4 - 2 phi_2,4
a42 = a43 = phi_2,4
a52 = a53 = (1/2)phi_2,5 - phi_3,4 + (1/4)phi_2,4 - (1/2)phi_3,5
a54 = (1/4)phi_2,5 - a52
a51 = (1/2)phi_1,5 - 2a52 - a54

b1 = phi_1(z)-3phi_2(z)+4phi_3(z)
b2 = b3 = 0
b4 = -phi_2(z)+4phi_3(z)
b5 = 4phi_2(z)-8phi_3(z).
```

This is the source's (5.19), not a generic promise to “add another solver.” Validate its row sums, zero-operator Runge–Kutta order conditions, coefficient stability, and convergence on genuinely nonautonomous manufactured cases. The fifth stage returns to the half-step time after the fourth stage sampled the endpoint. Force providers therefore must be pure with respect to requested times and support nonmonotone evaluation order. [S2, (5.19)]

Additional coefficient fixtures are `sum_j a_ij=c_i phi_1(c_i z)` for rows 2–5, `sum_j a_ij c_j=c_i^2 phi_2(c_i z)` for rows 4–5, and `b1+b4+b5=phi_1(z)`. These identities have now been checked symbolically for nonzero `z`, in addition to the eight zero-operator order conditions. They are necessary fixtures, not a proof of every stiff-order condition. Evaluate `phi_j(z/2)` with the same argument-based stable policy as `phi_j(z)`. Python binary64 implementations were compared with 75-digit references on both sides of `z=-50` and `z=-1`, including half arguments. Rust coefficient implementation tests remain required.

### 5.4 Local indicators and scheduling

The reference attempt computes one full step and two half steps from the same committed state. It proposes the fine result without extrapolation. Initially use the **raw discrepancy** `u_fine-u_coarse` as the empirical local indicator in both velocity and vorticity norms. It is not a certified error bound.

A Richardson divisor `2^p-1` is allowed only with an `OrderEvidence` record covering the current method, norm, grid, timestep regime, and time window. A fitted pilot order cannot be frozen for every later concentrating interval. Fixed-step tests use at least four step sizes when order reduction is in question; adaptive studies refine tolerance and maximum step independently. Agreement between the two temporal methods is additional evidence, not a proof if both share an unresolved input or spatial operator.

The initial pilot may use local velocity `rtol=1e-7`, vorticity `rtol=1e-6`, explicit absolute floors `1e-10`, and `dt<=0.05 tau` for the manufactured case. These are declared starting settings, not tested universal limits. The advective guard is `dt max_x sum_i |u_i| kmax_i <= C_adv`, initially `C_adv=0.3` pending measured tests. Force events and derivative bounds may impose a smaller step. The scheduler belongs outside the core; it has a finite retry cap and cannot repair a spatial/input failure by retrying indefinitely.

### 5.5 Time representation

The bounded reference profile uses an exact dyadic tick clock. Its plan fixes a power-of-two quantum `2^e` and a `u128` target count. The state stores `elapsed_ticks` and `remaining_ticks` separately, with checked integer arithmetic enforcing `elapsed_ticks+remaining_ticks=target_ticks`. A step adds its tick count to elapsed and subtracts it from remaining. For full-step/two-half-step CM or HO comparisons, the requested count must be divisible by four so every stage time is integral in this quantum. All endpoint and stage counts are formed directly from the committed clock.

The caller owns step quantization; the core either attempts the exact requested tick interval or rejects it. It never silently rounds a requested physical time. Refuse overflow, negative/zero steps, an exhausted positive pre-singularity distance or an unrepresentable stage. The tick quantum and any exact migration are part of the numerical plan and checkpoint. A fixed `u128` tick span is finite: a separate exponent does not permit an unlimited number of binary levels between the target and smallest remaining time. Return `ClockCapacityExceeded` when the planned range cannot represent the requested experiment.

Force providers read elapsed and remaining independently. Startup uses the elapsed count; the similarity coordinate uses the remaining count. Convert each count and scale separately into the provider's arithmetic, with an accuracy record. Never reconstruct `tau` by subtracting a rounded elapsed time from the target. A positive exact clock value can still be unrepresentable or insufficiently accurate in a binary64 force implementation; that is an arithmetic/provider failure rather than permission to use zero.

Two binary64 accumulators are not an exact substitute. In the executed fixture, adding a dyadic half-ULP step to elapsed rounds while remaining is still exactly representable; their floating sum appears to equal the target even though their exact rational sum differs. Exactness therefore applies to the integer tick clock, not to arbitrary dyadic additions in floating point.

The construction compiler may use arbitrary-exponent scaled or logarithmic quantities in its separate research profile. Accurate absolute auxiliary phase reduction and full force evaluation remain additional obligations. Such a compiler representation does not silently enlarge the bounded runtime clock's capacity. A logarithmic-time PDE implementation would require explicitly transformed equations and independent validation.

### 5.6 Commit semantics

`try_advance` attempts exactly the requested interval. It reads an immutable committed state and overwrites separate workspace and candidate buffers. Rejection leaves the committed Fourier coefficients, exact clock, accumulated diagnostics, count and lineage unchanged. Candidate storage and acceptance metadata are private to the implementation.

Commit takes mutable references to the committed and candidate holders and consumes a private, non-clonable `AcceptedAttempt`. Before any mutation, validate the base digest/epoch, plan epoch, candidate generation, layout/capacity and acceptance identity. Then exchange the two preallocated state payloads using `mem::swap`; the candidate holder receives the former committed storage for the next attempt. Invalidate its prior acceptance metadata. No vector is cloned or dropped as an incidental replacement, and an error path cannot perform a partial swap. A by-value Rust parameter does not itself imply allocation; this explicit exchange closes the previously unspecified buffer-return contract. [S3]

The core performs no allocation, I/O, internal retry or hidden partial advance. Allocation-free commit includes diagnostic/history buffers in the bounded state: preallocate fixed-capacity accumulators or have the CLI persist a report outside commit. Dynamic unbounded history append is not part of the core transaction. The implementation test records allocation/deallocation counts and buffer pointers over repeated accepted and rejected attempts, and verifies complete rollback for stale tokens.

The plan fixes maximum nonlinear evaluations, force work and scratch. A full-step/two-half-step CM attempt reserves twelve nonlinear evaluations before reuse optimization: 108 scalar transforms, plus declared force/diagnostic work. HO uses five stages per step and receives its own bound. Unknown callback costs require an explicitly unbounded research profile. An operation-count budget is not a hard wall-clock guarantee.

## 6. Independent experiment and convergence protocol

### 6.1 Identity and three separate data paths

`problem_id` hashes canonical mathematical definitions: equation, viscosity, geometry, initial field, profiles, cutoff functions, constants, coordinate mapping, target time, and any infinite-sequence selection rule. It excludes runtime grid and approximation precision. Exact rational benchmark constants and defining formulas are stored canonically; time constants are dyadic while the v2 radii use rational denominators 10 and 50. Diagnostic settings and numerical screening heuristics have separate identities and are excluded from the mathematical problem hash. For implicitly defined source coefficients, the definition and unique selection rule identify the mathematical quantity; rounded approximations are artifacts, not new exact coefficients.

`artifact_id` additionally records evaluator version, numeric approximation, precision, grid, coverage interval, and accuracy classification. Refining an artifact for the same problem must preserve compatible enclosures or pass a stated empirical comparison. Hash agreement alone gives no error bound.

The prescribed-force provider cannot receive the evolving velocity. The integrator has no reference-velocity API. The comparison harness can read both paths and cannot modify either trajectory. A fixed continuum residual is a legitimate way to prescribe a manufactured force; constructing a discrete residual from the evolving numerical field to cancel its error is prohibited.

### 6.2 Successive intervals

For the default member, let `delta_0=T_star`, `delta_k=delta_0 2^-k`, and `T_k=T_star-delta_k`, beginning with `k=1`. Every claimed window is `[0,T_k]` from rest. Each branch may continue from its own accepted checkpoint while earlier history remains qualified. A coarse-to-fine restart is a transfer experiment with inherited error, not a substitute for a fine branch integrated from rest.

Before extending, verify force coverage, resource feasibility, and all earlier accepted inputs. Changing the target force outside the old error budget invalidates affected history. Discovery that an earlier pulse, derivative, or force scale was unresolved invalidates all descendant windows; rerun from rest or a checkpoint before the first unsupported interval. Reference-seeded starts are local tests under a different lineage label and cannot advance the independent frontier.

For v2, the individual-grid screening convention `N sqrt(T_star 2^-k)>=12` counts cells across the equatorial swirl-peak radius, not its diameter. It allows at most `k=3` on `N=512` and `k=5` on `N=1024`; the latter reserves about 636.71 GiB before scratch. At `N=128` even the first endpoint fails this particular screen. These are heuristic reach estimates, never promised accepted windows or universal hardware ceilings. The coarser branches of a three-grid comparison may restrict the qualified frontier further. The implementation chooses its actual grid cap through memory preflight and measured convergence.

### 6.3 Minimum refinement family

| Channel | Required comparison | Interpretation |
|---|---|---|
| Space | Three increasing grids, initially `N`, `3N/2`, `2N`, with subordinate time/input errors | Resolved full-field agreement and scale sensitivity |
| Time | At least three temporal settings on the finest grid, additional order study when needed, and HO comparison | Temporal accuracy for the current window and norms |
| Force | Two increasing evaluator precisions and force sampling resolutions; profile orders when applicable | Input approximation and aliasing sensitivity |
| Integrator arithmetic | Same problem under independently improved arithmetic, or an explicitly qualified roundoff-error analysis plus sensitivity study | State/FFT/reduction error; rounding only the force does not satisfy this channel |
| Reference | Independently evaluated `u_S,p_S` at synchronized physical times | Manufactured-solution tracking error, including reference accuracy |
| Transfer | Direct fine branch from rest versus declared coarse-to-fine branch, when used | Inherited state and transfer errors |
| Perturbation | Fixed divergence-free perturbation with a distinct problem identifier, refined independently | Sensitivity/instability of the flow; does not excuse tracking error in the original problem |

Run combinations may share common branches. When two channels interact, refine them jointly rather than assuming separability. The first CPU implementation may initially lack a sufficiently strong arithmetic study. Such a run reports that missing channel and cannot receive the fully qualified window status.

Pad the coarse field onto the full fine band for primary differences; report common-band and newly resolved-band contributions separately. Use volume `L2`, Fourier `H1`, vorticity norms, local physical-space errors, oversampled maxima, and concentration location. In the half spectrum, use correct multiplicities. Do not recenter, rotate, or phase-align before the primary comparison; optional aligned comparisons disclose the transformation.

For v2 define nominal interior diagnostics on `|eta|<=1/2`: core `0<=X<=1/2`, annulus `1/2<X<=8`. At every comparison time intersect each region with the mathematical set `c_x=1`. Report its physical-volume coverage relative to the nominal region, and use an explicit `RegionEmpty` status if empty. Also report the cutoff collar `0<c_x<1` and startup interval separately. Masks use the declared mathematical cutoff boundary rather than an underflow-created apparent plateau. No mask is allowed to hide a failed global or collar comparison.

For the spherical cutoff at fixed remaining time, coverage is a one-dimensional quadrature in `eta`. Let `q=tau/(1-eta^2)`, `z=eta q^D`, `X_limit=(R_in^2-z^2)/(2q)` and weight `w=q^(1+D)(1-2h eta^2)/(1-eta^2)`. For nominal radial bounds `[X_low,X_high]`, integrate `w*clamp(X_limit-X_low,0,X_high-X_low)` over `[-1/2,1/2]` and divide by `(X_high-X_low)*integral(w)`. The common `2pi` cancels. Give this quadrature its own refinement or enclosure label.

The geometry check shows that the entire nominal annulus fits inside `R_in=0.30` from the first endpoint onward: at its worst corner in the first endpoint, squared spherical radius is `0.08818023364... < 0.09`. This is not a claim of full coverage over the entire first window from rest; early times still require the intersection and coverage report. Without the finite `eta` range, checking only `z=0` would not establish three-dimensional coverage.

These regions describe measurements, not constraints on the evolving state. Report local absolute errors and relative errors with a declared floor, initially `1e-3` times the relevant reference peak. A small global error cannot substitute for failed local derivative diagnostics.

### 6.3.1 Concrete arithmetic comparison path

The first independent arithmetic reference is a small-grid Python/mpmath implementation of the same semidiscrete problem at 80 and 120 decimal digits. Use `N=4,8,12` as resource-qualified fixtures, normalized full complex Fourier storage, independent separable direct DFT sums, strict Nyquist removal, three-halves padding, projection, rotational products, and the same CM and HO stage times. Construct constants from integers or rational/string data, not previously rounded binary64 values. Use isolated precision contexts or sequential precision runs. The reference is a test dependency, not a runtime dependency. [S4]

Compare isolated transforms/products/stages, complete full-step/two-half-step attempts, and short trajectories of smooth multi-mode forced fixtures. The 80-to-120-digit change must be subordinate to the recorded binary64 discrepancy. To isolate integration arithmetic, supply the same exact input state and prescribed force definition and control their evaluation error independently; compare a separate force-refinement experiment rather than mixing it into this channel. Persist full-band coefficient discrepancies and precision/input identities.

This small-grid path qualifies the tested arithmetic operations and trajectories. It does not establish roundoff control on a concentrating `512^3` trajectory. A full empirical window additionally needs a current-grid improved-arithmetic comparison or a quantitatively qualified arithmetic-error analysis plus a refined sensitivity study. Otherwise report `ArithmeticEvidenceMissing` and stop short of the fully qualified window label. No arbitrary-precision FFT implementation is presumed to exist.

### 6.4 Acceptance and stopping

Before a growth study, freeze an observable-specific tolerance file. A pilot starting target is global velocity relative error `1e-4`, `H1` and vorticity relative error `1e-3`, absolute floors `1e-8`, and separate local peak/location tolerances in the case's nondimensional units. These are experimental targets, not executed results. Budgets must cover reference, time, space, force, arithmetic, and transfer contributions; uncertainty in an unresolved channel cannot be allocated a value of zero.

`WindowAcceptedEmpirically` requires all mandatory channels below their declared budgets, refinement consistent with convergence or a documented subordinate error floor, and reference errors below every required threshold on the tested time set. Report exactly that time set, its refinement, and off-stage residual checks. A claim over the entire continuous interval additionally needs control of interpolation/reconstruction between samples; reserve `WindowAcceptedWithEnclosures` for actual slab-wide bounds and controlled arithmetic.

A growing perturbed trajectory may be an interesting instability result after independent refinement. It cannot qualify an original tracking window whose reference discrepancy exceeds tolerance. End the tracking claim and report the sensitivity experiment separately.

Stopping statuses include `ConvergenceInconclusive`, `ResolutionLimited`, `ForcingUnresolved`, `ArithmeticResolutionLimited`, `ReferenceUnresolved`, `TimeStagnation`, `BudgetExceeded`, and `Invalidated`. Numerical overflow or a growing sampled maximum is never `BlowupConfirmed`. Local `StepAccepted` is not a window-convergence decision.

### 6.5 Balances, continuous defect, and retained history

Record mean momentum, divergence, Hermitian defects, energy, enstrophy, viscous dissipation, forcing work, spectra, directional velocity/vorticity tails, and oversampled maxima. Independently check

```
d(1/2 <|u|^2>)/dt = <u dot f> - nu <|omega|^2>
d(1/2 <|omega|^2>)/dt = <omega dot S omega>
                              -nu <|grad omega|^2> + <omega dot curl f>.
```

Balance quadrature must be of sufficient order, with its own refinement. Accepted start, midpoint and endpoint values support Simpson quadrature; stage values alone are not silently treated as an exact dense solution.

For a continuous divergence-free reconstruction `v`, evaluate

```
r_v = partial_t v + P((v dot grad)v) - nu Laplacian(v) - P f_eval
```

through an independently implemented conservative product path. A degree-five Hermite interpolant through start/midpoint/endpoint states and derivatives is the initial temporal reconstruction. Probe off-stage points as well as nodes and refine the reconstruction. A `2N` diagnostic grid represents the full quadratic product under the stated Nyquist convention; it has a separate allocation budget. Forcing above that band still needs its own bound or measured convergence. Residuals only at interpolation nodes may vanish by construction and are inadequate.

For smooth target `U`, let `e=v-U`, `a(t)>=||sym grad U||_infinity`, and `b(t)>=||r_v+P(f_eval-f_target)||_2`. The periodic divergence-free energy estimate gives

```
||e(t)||_2 <= exp(integral_(t0)^t a) ||e(t0)||_2
           + integral_(t0)^t exp(integral_s^t a) b(s) ds.
```

Earlier error and transfer uncertainty enter the first term and are never reset at interval boundaries. Sampled strain maxima are not rigorous upper bounds. Even with an exact reference formula, the exponential may make this estimate uninformative. Report that outcome; do not call the estimate a peak-velocity or vorticity certificate.

## 7. Rust architecture and public contracts

Use module boundaries for domain/state, Fourier operations, integrators, diagnostics, cases, compiler, and CLI, combining them into fewer crates initially if useful. The public `navier-runtime` facade owns stable public types. Source-specific compilation is optional and downstream from the generic core. Public builds, tests and examples must work without a compiler artifact or a Niva checkout.

The following signatures are design contracts, not compiled APIs:

```rust
pub trait PrescribedForce: Send + Sync {
    fn identity(&self) -> ProblemId;
    fn plan(&self, request: &ForcePlanRequest)
        -> Result<ForcePlan, ForceError>;
    fn evaluate(&self, time: SimulationTime, plan: &ForcePlan,
                out: &mut VectorFieldMut<'_>, scratch: &mut ForceScratch)
        -> Result<ForceEvaluationReport, ForceError>;
}

pub trait ReferenceEvaluator: Send + Sync {
    fn evaluate(&self, request: &ReferenceRequest,
                out: &mut ReferenceFieldsMut<'_>, scratch: &mut ReferenceScratch)
        -> Result<EvaluationAccuracy, ReferenceError>;
}

pub fn try_advance(plan: &SolverPlan, committed: &SpectralState,
                   force: &dyn PrescribedForce, request: &StepRequest,
                   workspace: &mut StepWorkspace, candidate: &mut CandidateState)
    -> Result<StepReport, SolverError>;

pub fn commit_candidate(plan: &SolverPlan, committed: &mut SpectralState,
                        candidate: &mut CandidateState, accepted: AcceptedAttempt)
    -> Result<(), CommitError>;
```

`ForcePlanRequest` includes the physical evaluation grid, coverage interval, derivative demands, approximation tolerance, arithmetic policy, and work/memory caps. `ForcePlan` pins those inputs and declares transforms, maximum root/jet iterations or other bounded work, scratch, temporal events, and a coverage/accuracy classification. `ForceEvaluationReport` identifies the artifact, actual work, force and requested derivative errors, and unsupported limits. Estimates and rigorous enclosures are distinct enum variants, never an ambiguous boolean named “accurate.”

`ReferenceRequest` declares physical time, points/grid, derivative order, pressure gauge, and accuracy. It is only available to verification code. `StepRequest` specifies the exact positive interval, base-state digest, plan epoch, local tolerances and method. `StepReport` contains acceptance or rejection, attempted interval, norm indicators, diagnostics, force reports and the limiting channel. It does not contain scheduler advice. `AcceptedAttempt` is issued only by a successful attempt and is consumed at commit.

All dimensions, finite values, positive lengths/viscosity, layout, conjugacy, non-overlapping buffers, base-state epoch and scratch capacity are validated. User input errors return structured errors, not panics. Mutable workspaces belong to individual trajectories. Plans are immutable; `Send + Sync` does not permit sharing mutable trajectory scratch. FFI boundaries, if later added, validate ownership and prevent unwinding into foreign code.

### 7.1 Checkpoints and reproducibility

Save the complete Fourier state, physical clock, mathematical problem identity, force artifact, evaluator coverage, plan/layout versions, method and controller state, accepted-step count, accumulated balance quadrature, error/history records, numerical epoch and lineage. Recreate FFT plans without hidden state affecting the mathematical result. A checkpoint round trip must reproduce the next accepted/rejected attempt under the same recorded execution profile.

A deterministic CPU profile fixes compiler, dependencies, transform selection, summation order, and reductions. Its bitwise promise is scoped to that profile. Cross-hardware numerical agreement is a separate test; Rust and `f64` alone do not establish portable bitwise equivalence. Wall-clock telemetry is excluded from numerical decisions and deterministic state hashes.

The CLI owns input parsing, retries, checkpoint/output I/O, comparison and invalidation. Proposed commands are `case write`, `run`, `resume`, `experiment compare`, `experiment extend`, and `construction audit`. They are not currently installed commands. The implementation plan requires concrete schemas and an end-to-end example before a release.

## 8. Resource model and feasibility gates

For a cubic grid, let `H=N^2(N/2+1)`, `M=3N/2`, and `Hp=M^2(M/2+1)`. One retained three-component complex half-spectrum is `48H` bytes. One padded real vector is `24M^3` bytes. The initial conservative reservation is:

| Allocation class | Reservation |
|---|---:|
| Committed/candidate, stage, nonlinear and comparison vectors | `12 * 48H` |
| Padded real velocity, curl, and product/force workspace | `3 * 24M^3` |
| Padded complex vector staging | `48Hp` |
| Six real retained scalar coefficient tables | `6 * 8H` |

The padded complex vector buffer can be reused sequentially: transform velocity into its real buffer, then curl into its real buffer, then transform the product. The twelve retained-vector reservation must be accompanied by a concrete lifetime schedule in implementation; any extra retained storage is added to the ledger before allocation. A proposed schedule may reuse buffers after their last stage dependency, but cannot overwrite committed input or the saved coarse comparison.

| Retained `N` | One retained vector, GiB | One padded real vector, GiB | Reserved base, GiB |
|---:|---:|---:|---:|
| 128 | 0.04761 | 0.15820 | 1.25336 |
| 256 | 0.37793 | 1.26563 | 9.98218 |
| 512 | 3.01172 | 10.12500 | 79.67871 |
| 1024 | 24.04688 | 81.00000 | 636.71484 |

These numbers were computed by the verification script. They exclude FFT plans/scratch, provider jets and caches, wavenumber/index metadata, I/O buffers, allocator overhead, and independent diagnostic grids. They are reservations, not measured peaks or unavoidable lower bounds. Preflight reports every allocation class, rejects overflow in size arithmetic, and refuses a plan exceeding the user's cap. Full-band diagnostics may stream or run offline under a separate declared memory cap; neither option may drop modes from the reported norm.

The explicitly defined twelve-cell swirl-peak-radius screen in Section 6.2 and a weak spectral tail are only initial screening heuristics. The first grid and endpoint are chosen by memory preflight and the explicit member's scales, then accepted or rejected by refinement. No `512^3` or `1024^3` success is promised. Smaller-scale backends are considered only after a quantified failure and require complete transformed PDE operators, axis/boundary treatment, global pressure coupling, transfers, and overlap evidence.

For the literal construction, similarity coordinates and large exponents can represent mathematical parameters, but do not guarantee that a full independent PDE state or prescribed force can be resolved. The compiler must produce actual phase/derivative bounds and a representation-specific estimate before such an experiment is admitted. No generic adaptive-backend placeholder closes this gate.

## 9. Verification and release conditions

A release requires direct-DFT and explicit-convolution operator fixtures; zero-mode forcing and pressure tests; pure diffusion and Beltrami decay; a nonautonomous smooth manufactured solution; force-aliasing negative controls; temporal-order studies including HO; `similarity-mms-v2` derivative/startup/localization checks; from-rest grid/time/input refinements; and rollback/checkpoint/independent-trajectory tests.

Mandatory adversarial controls alter a high mode, lower force precision, introduce force sampling aliases, change an input between interval extensions, and restart from a reference field. The harness must detect or correctly relabel each change. Removing an active pulse is a source-compiler test once pulses exist; it is not pretended to be exercised by the pulse-free manufactured case.

The actual checks already executed are in [navier-runtime-verification-results.json](navier-runtime-verification-results.json), with the reproducible [script](navier-runtime-verification.py). Revision 0.7 adds the four-question source screens, nonzero-operator HO identities, binary64 coefficient junctions, cutoff/endpoint geometry and a floating-clock counterexample. They validate selected algebra, arithmetic and geometry; buffer swapping, the high-precision DFT path and the PDE experiment remain unimplemented. The ordered work and exit conditions are in [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md). There is presently no accepted PDE window, compiled Rust implementation, or admitted source instance.

## References

- **S1:** OpenAI, *Finite Time Blowup for Navier–Stokes*, supplied [manuscript](https://cdn.openai.com/pdf/32d9f210-8b73-45e0-91bc-82a30aef8a9a/navier-stokes.pdf). Targeted equation/lemma references identify the source dependence; this design is not a proof audit.
- **S2:** M. Hochbruck and A. Ostermann, *Explicit Exponential Runge–Kutta Methods for Semilinear Parabolic Problems*, [author-hosted manuscript](https://publikationen.bibliothek.kit.edu/1000042061/3153602), especially (5.19) and the order-reduction discussion.
- The manufactured profiles, cutoff choice, pressure localization, engineering interfaces and acceptance policies above are proposed project definitions or explicitly derived identities. They are not assertions that those exact choices appear in the manuscript.

- **S3:** Rust standard library, [mem::swap](https://doc.rust-lang.org/std/mem/fn.swap.html), exchange of two mutable values. The allocation and transaction contract above is a proposed project requirement.
- **S4:** mpmath, [arbitrary-precision arithmetic and input precision](https://mpmath.org/doc/current/basics.html). The DFT comparison algorithm and acceptance scope above are proposed project tests.

<!-- END COMPLETE_DESIGN.md -->

---

# Input 4: CONSTRUCTION_LEDGER.md

<!-- BEGIN CONSTRUCTION_LEDGER.md -->

# navier-runtime: source and construction ledger

Revision 0.8 · 8 September 2026

This ledger separates known equations, chosen benchmark definitions, executable design, actual computations, and unresolved source extraction. It does not certify an admissible instance of the paper. The governing engineering specification is [COMPLETE_DESIGN.md](COMPLETE_DESIGN.md); review corrections are in [ADVERSARIAL_REVIEW.md](ADVERSARIAL_REVIEW.md).

## 0. Four-question feasibility screen

The revision-0.7 source extraction is retained in [SOURCE_FEASIBILITY.md](SOURCE_FEASIBILITY.md). The current release proceeds with the adopted revision-0.7 runtime and `similarity-mms-v2`; source compilation is optional mathematics verification and is not a prerequisite. Literal-source reproduction is excluded from the current release scope.

| Question | Extraction completed in revision 0.7 | Exact remaining obligation |
|---|---|---|
| Actual `q_star` versus starting band | Lemma 7.1 explicit sufficient test; common-domain dependence in Lemma 9.7/Prop. 9.9; quarter band fails that test by about `1.33e20`; relaxed conservative screen passes at `ell=11217670` | Evaluate actual profile/frame/covariance/inverse margins; do not equate the screen with `q_star` |
| How `S_star=ell^2` enters work | Mesh `S^-3`; bounded point overlap; chart/radial count factors `S^9/S^3`; pulse interval and envelope conversions | Localized support enumeration, lazy-query cost, radial/auxiliary inverse work and material physical variation |
| Phase and pullback scales | Exact `v_r,v_t,d_r,i,c_i,M_i` and phase-normal formula; relaxed scalar map values computed | Actual background values/derivatives, cone choices, integer rounding, phase extrema and error budgets |
| Active background/correction orders | Cutoff selection inequality, gains `2hn` and `hj/10`, finite support test and doubling bound | Actual coefficient bounds, immutable `c_n,a_j` sequences, initialization activation and interval-wide intersections |

Proposition 10.1's radial cutoff freedom is source checked. Add physical `r_cut,z_cut`, cutoff functions, activation and scaling to the realization identity. Keep `r_cut` distinct from the Section 6 auxiliary radius `r_aux`. A smaller onset box does not remove global profile obligations or the eventual entry of the shrinking outer annulus. The literal target across unspecified representations remains `FeasibilityUnestablished`. The declared explicit subdivision plan has the separate gate below.

### 0.1 Scoped feasibility gates and release decision

The machine-readable [source-feasibility-policy.json](source-feasibility-policy.json) records `FeasibilityExcluded` for `explicit-slow-mesh-v1`: chart segment length `1/4`, spacing at most `ell^-6`, maximum 1,024 stored intervals per axis. The required interval count is `ceil(ell^6/4)`: about `7.3718e29` at band 119749 and `4.9815e41` at band 11217670. These counts increase with band. The cap is a declared policy; active support, hardware capacity and universal field complexity have not been inferred from it.

The general literal-target gate is `FeasibilityUnestablished`, and no direct reproduction is in the current release plan. `MathematicsVerificationOnly` is the compiler's present scope. The optional averaged-stress surrogate is `SeparateDecisionDeferred` with a distinct model identity. Neither a compiler check nor a surrogate may advance the literal-source frontier.

The review's uniform order-one pulse argument is rejected: the pure factor is `sqrt(epsilon)`, which is at most one half for `epsilon<=1/4`; omitted profile/logarithmic/weight factors also prevent interpreting this as an absolute pulse bound. Full support extent is at most `2ell^-6`; auxiliary traversal and slow cutoff durations are different constraints. The heuristic `k/ell^6` crossing is recorded as arithmetic, not an admission condition. See the source report for equations, source locations and required error obligations.

## 1. Status vocabulary and provenance

| Status | Meaning |
|---|---|
| Source checked | The identified text or displayed equation was inspected in this task; the whole proof is not thereby audited |
| Algebra checked | An identity was derived and checked by the supplied executable script |
| Specified | Inputs, algorithm and acceptance criteria are written; no implementation success is implied |
| Reviewer reported | Present in the supplied reviews, without independent verification of the relevant source file in this task |
| Extraction pending | Required equations, constants or numerical algorithm have not been completely extracted |
| Computation pending | The algorithm is defined, but no resulting numerical object or qualifying run exists |

The primary NS PDF is identified by its supplied URL, title and inspected sections. Browser metadata reports 166 pages. A PDF byte checksum is still absent because the local download was blocked. A release-quality provenance record must add the actual bytes' SHA-256; do not invent it from the URL or the review's document description.

Revision 0.5 reports Euler commit `0e9897ca7058213f47030e218a63a9e50f63e5bb` and Lean commit `8937a8f4cbc7abaab5e9e97d1cc7f5d2319d9538`. Neither exact commit's contents were successfully retrieved here. The public formalization landing page was read, but no Lean build was run. Detailed license/file counts and comparator implementation remain reviewer-reported. The revision-0.7 review also reports reading `NavierStokes/SlotColoring.lean` at the pinned Lean commit; attempts to retrieve that file here failed. Its mesh agrees with the PDF, which was independently checked. This is not an independent Lean-file verification. The Euler project is excluded as an NS numerical oracle. The design requires no copied code, prose or figures from that repository.

## 2. Quantities that must not be conflated

| Quantity | Definition or record | Required interpretation |
|---|---|---|
| Smallness exponent | `h < min(1/100,lambda,exp(-T_d))` | Binding source inequality, not an arbitrary accessible exponent |
| Dyadic band | `Q=2^-ell` | Exact band label, distinct from local coordinate `q` even when comparable |
| Expansion parameter | `epsilon=Q^h` | Smallness and asymptotic regime |
| Integer carrier | `k=ceil(epsilon^-1/2)` | Already at least two whenever `0<epsilon<1` |
| Normalized growth | `tau^-h` | A dimensionless power-law factor |
| Reynolds diagnostic | Chosen physical speed times chosen physical length divided by `nu` | Requires profile constants, location/norm convention, and scaling; the source states comparability to the power law |
| Annulus ratio | `sqrt(X_b/X_a)` | Scale-invariant geometry, not by itself a phase-resolution lower bound for every representation |
| Numeric precision | Significand, exponent range, phase and cancellation budgets | Tiny time representation and accurate physical force are separate obligations |
| Source instance | All mathematical choices and infinite-sequence rules frozen and admitted | Not an arbitrary finite residual or an accessible manufactured member |

The relaxed arithmetic in the supplied verification results is a **lower-envelope illustration**, obtained at boundary values such as `M_d=0`; those values are not presented as satisfying the source's “large” conditions. It does not produce a concrete parameter set or a finite resource upper bound.

## 3. Source-to-algorithm ledger

Rows identify dependencies rather than pretending a theorem citation is an implementation. Every accepted source object eventually needs an exact definition, numerical artifact, admissibility record, approximation/derivative bounds, and dependency digest.

| ID | Source object | Representation and required calculation | Dependencies | Present status |
|---|---|---|---|---|
| S00 | Theorem 1.1: fixed smooth compact force, rest initial condition, finite target time | Immutable equation/force identity; no trajectory feedback; distinguish the source claim from numerical evidence | PDF version | Source statement checked; no source force produced |
| S01 | Similarity geometry, (3.2), (4.1), Lemma 4.1 | Positive root of `q-z^2 q^(2h)=tau`; scaled exponents; implicit derivatives with residual/error checks; Cartesian axis evaluation | `h`, mapping, positive `tau` | Equations checked; prototype algebra checked; bounded Rust evaluator pending |
| S02 | Full parameter hierarchy, Lemma 4.8, Prop. 4.10, proof of Theorem 4.6 | Dependency graph of every strict inequality; interval margins and exact candidate selection; record `M_d,T_d,P_*,h,C,Lambda` and remaining constants | S01, outer/core profile estimates | Binding restriction and relaxed arithmetic checked; full constants and admissible candidate absent |
| S03 | Exterior profiles and pressure datum, Appendix A, Lemma 4.8 | Actual profile equations and boundary/asymptotic data; log-radius representation, controlled quadrature and derivative tails | S02 | Leading exterior form reported in review; complete numerical recipe and artifacts pending |
| S04 | Axis analytic solve, Appendix B, Prop. 4.10 | Analytic Cauchy problem with axis data and interval of continuation; coefficient representation, analyticity radius, remainder and conditioning control | S02,S03 | Source dependence identified; implementation algorithm not yet validated; do not replace by an unconstrained ODE/interpolant |
| S05 | Collar/transition and shear realization, Appendices B/C | Smooth gluing, exact supports, derivative bounds, cone/stress realization and frequency choices | S03,S04,S02 | Extraction pending; no claim that a logarithmic radial grid alone makes this inexpensive |
| S06 | Five cumulative integrals, Lemmas 4.3/4.4; moment correction Lemma 4.7 and (4.42) | Functional identities in `eta`; bump supports and matrices; inverse norm, nonlinear mismatch/contraction condition, positivity and derivative enclosures | S03–S05 | Required smallness/invertibility issue checked; no solved source moment functions |
| S07 | Leading `E,U,V0,Pi`, Theorem 4.6 | Assemble profiles, verify all moments, core/exterior conditions and every positive cone margin over their whole domains | S02–S06 | Algebraic field map specified; source-admissible profiles absent |
| S08 | Background coefficients, Section 5 and Lemma 5.4, (5.34)–(5.40) | Extract coefficient recursion, inverses, derivative bounds, cutoff scale sequence and deterministic selection rule | S07 | Cutoff inequalities/gain and summation procedure extracted; coefficient bounds, actual recursion artifacts and numerical specimens pending |
| S09 | Auxiliary torus and support labels, Section 6 | Exact phase map, constants, label enumeration, collision/separation conditions and support geometry; auxiliary torus is not an extra physical spatial dimension | S02,S07,S08 | Map, mesh, bounded-overlap/count estimates and rectangle time scale extracted; complete numerical enumeration and support constants pending |
| S10 | Phase and pulse dynamics, (7.2)–(7.5) | Band labels, integer carrier, full phase coefficients and physical gradient; nonstationary viscous amplification, envelopes and covariance weights | S07,S09 | Full normal, displayed smallness test and envelope scales extracted; scalar relaxed screens computed; profile-dependent phase bounds and pulse evaluator absent |
| S11 | Oscillatory potentials and curl remainders, Section 7 | Divergence-free Cartesian reconstruction, nonlinear cross terms, all cutoff derivatives and residual error terms | S10 | (7.38) and Lemma 7.7 inspected: normalized curl identity and wave-class derivative loss checked; finite-band constants, complete evaluator and residual bounds pending; no pulse is implemented |
| S12 | Mean corrections and moment inverses, Section 8 | Explicit inverse operators, supports, moment-map solvability and error constants | S08,S11 | Extraction pending |
| S13 | Residual cycle and final sums, Section 9, Prop. 9.9, (9.20)–(9.21) | Recursion with computed constants, deterministic append-only prefix, residual norm hierarchy and cutoff sequence | S08–S12 | Common-domain proof, stage gain, final cutoff formula and initialization freedom inspected; actual coefficient/cycle artifacts absent |
| S14 | Localization and force extension, Section 10 | Full potential/pressure localization, derivative terms, support, zero initial interval, flatness and extension through target time | S13 | Prop. 10.1 and Lemmas 10.2–10.3 inspected: radial radius is free; endpoint/extension estimates retain global source dependencies; no source force artifact |
| S15 | Viscosity and periodic bridge, (10.22), Cor. 10.6 | Composed space/time/field map, shifted activation, disjoint periodic supports and mapped target time | S14 | PDF map inspected and algebra checked; source-specific support constants pending |
| S16 | Independent finite-window force/reference artifacts | Accurate continuum evaluation of the admitted source object, no input from evolving PDE state, common mathematical identity across refinements | S00–S15 | Gate specified, no artifact or admitted interval exists |

Sections/appendices listed as extraction pending have not been independently read in their entirety in this pass. Earlier targeted source checks and the review narrow their location; they do not remove the extraction obligation.

### 3.1 Parameter selection is a verified computation

Replace each “sufficiently small/large” choice by a deterministic search only **after** extracting all inequalities affected by that choice. The search records the exact rational candidate, dependency version, evaluated interval margin, norm and domain, precision, approximation order and proof/estimate classification. An interval containing zero requires further refinement or a reported unresolved condition. It does not count as a positive margin.

Choosing constants right to left in the paper's hierarchy is not enough when an inequality's actual bound is missing. That situation returns `MissingBound`, with the precise lemma/object and next calculation. Budget exhaustion returns `AdmissibilityUnresolved`, not evidence against the mathematical existence claim.

Function-valued moment corrections require a representation over `eta` and derivative control, not just a finite vector of point samples. An exact implicit function may be identified by its defining equation and uniqueness region; successive polynomial/interval approximations are numerical artifacts. Do not substitute rounded dyadic coefficients for the exact root while continuing to claim exact moment identities.

### 3.2 Scaled representations and conditioning

Separate exponent and mantissa for very small `q,tau`, or use log values for analysis with explicit conversion/derivative contracts. Factoring known powers out of field and residual expressions is desirable. It does not remove the need to bound the remaining cancellation or recover accurate oscillatory phase modulo its period. A source compiler must report both residual term magnitudes and the final absolute force error.

At the radial axis use regular Cartesian series, not `log X` at `X=0`. A composite representation needs an axis patch, log-radius patches, `eta` dependence, matching tolerances, and derivative enclosures. A finite log-radius extent by itself establishes no approximation order or stability guarantee for the analytic continuation problem.

### 3.3 Active terms and tails

Coverage records have three states:

```
ExactInactive: mathematical cutoff identically zero on the whole queried domain
Included: evaluated with its own approximation error record
OmittedWithBound: nonzero or uncertain term omitted with an explicit norm/derivative bound.
```

For `chi(s_j q)=0` when `s_j q>=1`, a proved `s_j q_min>=1` establishes exact inactivity, including derivatives for a smooth flat cutoff. Use the correctly mapped source coordinate and an interval-wide lower bound, not a sampled minimum. Uncertain activity is retained or bounded. Pulse-envelope tails, reconstruction remainders, and Fourier truncation are not automatically `ExactInactive` merely because the final correction index is finite.

Each certificate enumerates correction stages, pulse labels, harmonics, transition bands, and derivatives required by the evaluated quantity. Extending the prefix must prove added terms exactly inactive on the old covered interval or account for changed approximation error there and rerun affected trajectories. Source field residual tails retained in the prescribed force cannot simply be discarded during runtime-force generation.

### 3.4 Phase and seed ledger

Before a pulse can enter a literal PDE experiment, record its complete physical phase, spatial/time derivatives, envelope supports, seed amplitude, viscous damping, amplification estimate, mean covariance target and cross-interaction bounds. Test whether the chosen representation resolves these quantities with the declared tolerances. A generic `sqrt(q)/k` scale does not replace the actual phase-gradient calculation.

For any optional WKB comparison using `w=a cos(k Phi/epsilon)` and `xi=grad Phi`, the physical wavevector is `(k/epsilon)xi`. Its viscous damping therefore contains `nu(k/epsilon)^2|xi|^2` when `k,epsilon` are fixed within the local band. Revision 0.5's proposed ODE omits that denominator under its own stated phase convention. A different definition absorbing `1/epsilon` into `xi` must state that convention consistently. This dimensional/algebraic correction is not a derivation of the paper's full pulse system.

Floating-point roundoff is an error bound with correlations; it is not a guaranteed nonzero disturbance of a specified size in every mode. Source and numerical seeds must be projected onto the same unstable modes before their competition is quantified. Arithmetic, force and truncation perturbations retain distinct records.

## 4. Explicit manufactured member ledger

These are project definitions, not source-admissible choices.

| ID | Object | Concrete definition | Actual evidence |
|---|---|---|---|
| M01 | Parameters | `h=1/8`, `b=1/4`, `j0=1/32`, `nu=1`, unit periodic cube, `T_star=1/128` | Exact rational manifest specified in `similarity-mms-v2.json`; distinct problem from v1 |
| M02 | Profiles | `M=(eta+j0)X exp(-X)`, `U=M_X`, `F=b exp(-X)`, `E=sqrt(2X)F`, `Pi=-(b^2/2)exp(-2X)` | `U` moment and radial pressure identities checked symbolically |
| M03 | Coordinate evaluator | S01 admissible root; bounded bracket/Newton and implicit Taylor jets | Coordinate equation and error argument specified; selected algebra checked; bounded scalar/jet evaluator pending |
| M04 | Potential and swirl | `A_vec=(1/2)q^-A(eta+j0)exp(-X)(-y,x,0)`, `B_vec=b q^(-A-1/2)exp(-X)(-y,x,0)` | Streamfunction identity checked |
| M05 | Localization | Fixed spherical cutoff radii `3/10,21/50`; flat smooth time ramp of duration `T_star/4`; `u=curl(c A_vec)+c B_vec` | Cartesian divergence identity checked for arbitrary smooth radial scalar coefficients; numerical derivative checks pending |
| M06 | Pressure and force | `p=c^2 q^-2A Pi`; continuum residual under fixed positive viscosity | Algebraic recipe specified; no sampled force or PDE trajectory produced |
| M07 | Force derivatives | Physical jets degree three for force, degree four for derivatives; axis-safe expressions | Algorithm specified; accuracy/conditioning and scalar-root propagation tests pending |
| M08 | Experiment | Endpoints `T_star(1-2^-k)` from rest; fixed force; separate refinement channels and lineage | Interior mask/coverage formula specified; endpoint geometry checked; coverage quadrature unexecuted; no accepted window |

Tests of M01–M08 qualify only this member. Even a perfectly converged independent trajectory would not validate S02–S16 or establish smooth force at `T_star`.

## 5. Retained admission requirements for a future literal source-instance run

These requirements are retained for any future reopening of literal reproduction; they are not release work. The currently excluded explicit-mesh policy cannot pass by relabelling compiler checks or a manufactured solution. A different representation must provide a new scoped feasibility assessment. A failed sufficient estimate alone does not establish universal infeasibility.

`SourceFieldAdmitted` requires an immutable source version and realization, evaluated parameter margins, source profiles and all needed active objects, derivative/residual accounting, interval coverage and tails, and accurate localized force/pressure/reference artifacts. Numerical approximations carry a stated empirical or enclosed accuracy classification. A fully specified infinite selection rule may define future terms without materializing them, but the current interval's dependencies must be evaluated or bounded.

`SourceWindowAccepted` additionally requires a representation-specific work/memory/precision report and a successful nontrivial from-rest comparison family with all mandatory channels. A source field can be mathematically admitted before a trajectory is integrated; compilation and trajectory evidence are separate states. Neither label presently applies.

Each window reports whether initialization waves and which stages are materially active. Cutoffs chosen to delay stages are part of the mathematical identity. An early literal-source tracking window with no active pulses does not demonstrate the pulse mechanism. Changing those choices between extensions invalidates the lineage.

Until the required quantities are evaluated, retain `SourceCompilationUnresolved` or `FeasibilityUnestablished`. The manufactured case cannot satisfy missing source rows. The retained screens identify optional mathematical obligations. Runtime and benchmark implementation proceed without waiting for them.

## 6. Provenance completion and independent reproduction of these checks

The script [navier-runtime-verification.py](navier-runtime-verification.py) and its actual [JSON output](navier-runtime-verification-results.json) accompany the design. The review bundle records hashes for these design files and scripts; those hashes are unrelated to the currently absent PDF hash.

Pending provenance work is restricted to obtaining the exact public source bytes/commits, recording their checksums, reading any implementation source actually reused, and running any formal build whose result will be claimed. It does not require private Niva access. The project can implement the original manufactured benchmark and public numerical algorithms without copying Euler-repository code.

## Primary references

- OpenAI, [NS manuscript](https://cdn.openai.com/pdf/32d9f210-8b73-45e0-91bc-82a30aef8a9a/navier-stokes.pdf), source objects identified in the table.
- [NavierStokesAndEuler](https://github.com/openai/NavierStokesAndEuler), accessible public landing page; exact implementation/build status unverified here.
- [Euler visualization](https://github.com/pmocz/euler-blowup-viz), separate problem identified by the user; detailed repository contents remain reviewer-reported in this pass.

<!-- END CONSTRUCTION_LEDGER.md -->

---

# Input 5: IMPLEMENTATION_PLAN.md

<!-- BEGIN IMPLEMENTATION_PLAN.md -->

# navier-runtime: implementation and evidence plan

Revision 0.8 · 8 September 2026

Implement [COMPLETE_DESIGN.md](COMPLETE_DESIGN.md) in small verifiable increments. The first runtime and explicit manufactured benchmark have specified equations. The full source construction remains a separate workstream gated by [CONSTRUCTION_LEDGER.md](CONSTRUCTION_LEDGER.md); it is not hidden inside a generic force callback.

This plan describes future work. The executed work is the adversarial assessment, targeted source extraction, selected algebra/arithmetic/geometry checks, and design packaging. No Rust compilation or PDE integration has been performed.

Adopt the runtime specification at revision 0.7 without changes. The release path is the standalone runtime and `similarity-mms-v2`; literal-source PDE reproduction is excluded from this release. The compiler is optional mathematics verification, and a stress surrogate is a separate deferred decision. The [source report](SOURCE_FEASIBILITY.md) and [policy](source-feasibility-policy.json) retain precise feasibility scopes. Source extraction does not block the runtime. A proposed `128^3` Python pilot is an underresolution/implementation diagnostic, not an accepted concentrating window.

## 1. Ordered work packages

| ID | Dependencies | Concrete output | Required exit evidence |
|---|---|---|---|
| P00 | None | Adopt `COMPLETE_DESIGN.md` revision 0.7 and v2 manifest byte-for-byte; package review disposition and expanded verification at revision 0.8 | Both frozen SHA-256 values match; complete public packet and scoped feasibility policy verified |
| P00A | P00; optional, outside release critical path | Retained source extraction and scoped explicit-mesh policy | Mesh/amplitude/time corrections and heuristic checks recorded; exact source profiles, `q_star`, phase extrema and active counts unresolved |
| P00B | P00 | Python scalar/reference evaluator, implicit jets, and mpmath direct-DFT arithmetic fixtures | Derivative/cutoff/force accuracy; 80/120-digit comparisons; no PDE success inferred from algebra |
| P00C | P00B | Resource-approved Python PDE diagnostic pilot and force-sampling studies | From-rest trajectory and failure/accuracy reports; grid selected after preflight, `128^3` is a proposed diagnostic size only |
| P01 | P00B | Standalone public Cargo workspace, pinned toolchain/dependencies, license files, format/lint/test configuration | Fresh public-only checkout builds/tests; no Niva/private paths, services or credentials in any package, example or test |
| P02 | P01 | `Domain`, half-spectrum layout, state/clock, epochs, typed errors, allocation preflight | Overflow/invalid-input checks, exact mode indexing, conjugacy/Nyquist tests, exact u128 tick invariants/capacity errors, independent state ownership and budget refusal |
| P03 | P02 | FFT wrapper, padding/cropping, projection/curl, rotational nonlinear operator, pressure paths | Direct DFT and explicit convolution agreement on small grids; mean-mode and pressure-gradient-force fixtures; inviscid semidiscrete energy identity |
| P04 | P03 | Corrected CM ETDRK4 coefficients, stages, one-attempt transaction and external scheduler | Higher-precision coefficient tests, constant-source identity, exact modal diffusion, RK4 limit, actual-stage forcing, rejected-attempt rollback, stale-token refusal and zero allocations/deallocations over repeated swap commits |
| P05 | P02 | Independent scalar and jet evaluators for `similarity-mms-v2`; exact manifest serialization | Root brackets/residuals, mixed derivatives, axis limits, cutoff/startup/support checks, divergence and pressure identities, force cancellation/accuracy report |
| P06 | P03–P05 | Smooth generic manufactured tests and a first small full-3D from-rest concentrating pilot | Time/space refinement with known reference, independently checked force coefficients, documented unsuccessful cases and no reference state assignment |
| P07 | P04,P06 | HO five-stage tableau, independent temporal coefficient implementation, measured-order records | Nonzero-operator row identities, stable half arguments, tableau/zero-operator conditions, nonautonomous stiff test convergence, nonmonotone stage-time callback test, current-window CM/HO comparison |
| P08 | P03,P06 | Full-band/off-stage diagnostics, norm/pressure/balance comparisons, window verifier | High-mode and force-aliasing negative controls; independent residual reconstruction; no sampled maximum reported as rigorous supremum |
| P09 | P06–P08 | Separate force, arithmetic, reference and transfer studies; complete checkpoint/lineage schema | Correct channel attribution; same-problem comparisons; inherited error retained; invalidation replay tested |
| P10 | P07–P09 | First qualified endpoint and progressively closer endpoints of the explicit benchmark | Full mandatory comparison family below frozen tolerances; each endpoint's status and last resolved interval recorded |
| P11 | P01–P10 | Public CLI, examples, case manifests, release documentation and evidence artifacts | User can create/configure a case, run, inspect, checkpoint, resume and compare from the public package alone |
| P12 | P10,P11 | Small read-only visualization | Shows equation/case identity, accepted frontier and limitations; cannot change simulation or suppress failed evidence |

P05 can be developed alongside P03–P04 because its mathematical input is independent of the PDE engine. This is ordinary work dependency information, not a request to delegate this task or create agents.

## 2. Exact early fixtures

### 2.1 Operator fixtures

Use small grids for exact convolution and direct-DFT comparisons. Include wavevector pairs whose unpadded alias lands in the retained low band. Use nonzero mean velocity and mean force. Verify that a purely gradient force changes reconstructed pressure while its projected acceleration vanishes. Test negative frequencies and the half-spectrum Parseval weights on the stored `kz=0` plane and positive `kz` slices. Reject a nonzero Nyquist coefficient rather than silently accepting an inconsistent checkpoint.

For a divergence-free Fourier field, compare rotational and conservative projected nonlinear terms using independent product code. Check energy production of the inviscid semidiscrete nonlinear operator with a tolerance justified by transform arithmetic. This is an operator check; do not present it as a time-discrete energy-conservation theorem.

### 2.2 Time fixtures

Use a single decaying divergence-free Fourier mode, a uniform spatial mode with a time-dependent force, and a smooth divergence-free manufactured field involving multiple interacting spatial modes and independent time factors. Include a source with timescale comparable to the trial step so wrong stage times are detectable. Test stiff modes separately from the zero-diffusion RK4 limit.

Coefficient tests cover zero, both branch boundaries, large negative arguments, near-cancellation roots, and representable extremes. Report absolute conditioning-scaled errors at roots. The script accompanying this design supplies algebraic fixtures; the Rust implementation must compare its actual floating-point values against independently generated high-precision data.

For HO, record the requested force times: its last stage occurs at a half-step after an endpoint stage. A stateful force provider that assumes monotonically increasing requests must fail an explicit purity test.

Use preallocated committed/candidate payloads for transaction fixtures. Record their buffer pointers and allocator events through repeated accepted commits and rejected attempts; validate every digest/token before swapping. Exact clock tests use checked ticks, exercise quarter-step stage counts and capacity exhaustion, and retain the included counterexample showing that two binary64 dyadic accumulators can drift despite a passing floating sum assertion.

### 2.3 Manufactured-field fixtures

Evaluate the exact member at the origin, a small radial offset, generic interior points, the inner/outer spatial cutoff edges, startup ramp boundaries, and decreasing positive remaining times. Check the scalar-root bracket and implicit jet residual at each degree. Probe derivative agreement over step-size/precision refinements rather than using one finite-difference result as ground truth.

Check startup from exact rest and spatial support within the periodic cell. Use v2's exact rational parameters from `similarity-mms-v2.json`; changing a v1 case creates a new identity. Test nominal `|eta|<=1/2` masks, intersection with `c_x=1`, coverage quadrature and an explicit empty-region status, with startup and collar errors retained. Compare analytic force Fourier coefficients under increasing sampling grids before relying on a sampled spectral-tail diagnostic. Track the relative conditioning of individual momentum terms and their sum. The choice `h=1/8` is a benchmark input, not a paper-admissible exponent.

## 3. First PDE pilot and extension protocol

Begin with an allocation-approved small grid family, for example `32,48,64` only if the field and force are resolved sufficiently to make that experiment informative. Increase to `64,96,128` or larger when the measured spatial channel requires it. These are proposed pilot grids, not assertions of success or fixed release minimums.

All branches start at `t=0`, `u=0`, under the same immutable `similarity-mms-v2` manifest. The first target is `T_star/2`; later targets are `T_star(1-2^-k)`. A coarse preliminary branch may diagnose underresolution, but it is not a convergence result. Do not begin at a later reference field to make the first tracking plot look successful.

At a finest grid of `512^3`, v2 permits only three endpoints under the stated twelve-cells-per-peak-radius screen; coarser comparison branches can shorten that frontier. `1024^3` is not an assumed available upgrade: its base reservation already exceeds 636 GiB. A `128^3` pilot fails the first endpoint under this screen and is useful for implementation/underresolution diagnostics, not an automatic qualified window.

Choose and freeze an observable/tolerance configuration using the preliminary runs. Then run the specified spatial family with subordinate time and input error, the temporal family on the finest grid, force-evaluation refinement, reference refinement, and arithmetic study. Add coupled refinements when channels interact. The full-grid and local derivative errors decide acceptance; common-band-only agreement cannot qualify a window.

Persist raw snapshots, comparison times, norm values, spectra, force reports, branch manifests, invalidation records, and reasons for stopping. An empirical window report states its sampled times and reconstruction tests. A rigorous slab-wide label needs actual enclosures. Each extension inherits every earlier error and acceptance dependency.

## 4. Negative controls and expected responses

| Perturbation | What must respond | What must not be claimed |
|---|---|---|
| Inject a divergence-free high Fourier mode | Full fine-band and derivative-sensitive discrepancy; state provenance | Agreement after projection to the coarse band proves full agreement |
| Sample an unresolved high-frequency force whose aliases look smooth | Force oversampling/off-grid comparison, `ForcingUnresolved` | A small sampled tail alone proves the force resolved |
| Round the prescribed force to `f32` | Force conversion error and trajectory sensitivity under a distinct artifact or perturbation identity | This is a full integrator-arithmetic refinement |
| Change the force definition between interval extensions | Mathematical problem identity and lineage invalidation | The later run continues the original problem |
| Replace a state with the reference field | Independence instrumentation and `ReferenceSeededLocalTest` classification | The resulting branch is an independent from-rest reproduction |
| Restart fine integration from a coarse checkpoint | Transfer/inherited-error record and comparison with direct fine branch | Refining after lost history recovers the original fine trajectory |
| Force a rejected step or budget violation | Committed state/time/diagnostics unchanged; candidate cannot be committed | Scratch changes mean a partial physical advance occurred |
| Remove a pulse after a source evaluator actually exists | Source identity/coverage and targeted residual/covariance diagnostics | A pulse-free manufactured member has already tested the cascade |

A perturbation need not cause a dramatic global velocity change. Specify which mathematical input and diagnostic should change before running it. Physical sensitivity is separately tested by fixed perturbations refined over space, time and arithmetic; it does not waive original tracking tolerances.

## 5. Optional mathematics compiler and retained research requirements

This workstream is optional and limited to mathematics verification in this release. C0–C4 describe useful mathematical artifacts; C5–C6 are retained requirements for a future, separately scoped reopening of literal reproduction. They are not scheduled release work. Runtime packages P00B–P12 do not depend on C1–C6. The surrogate remains deferred and receives its own problem identity if commissioned.

| Stage | Concrete next work | Exit condition |
|---|---|---|
| C0 | Extract the four-question feasibility equations, evaluable relaxed screens and dependency graph; record free localization parameters | Completed in revision 0.7. Actual `q_star` margins, label/inverse work, phase extrema and stage counts remain explicit obligations of C1–C5. Obtain source bytes/commits separately for release provenance |
| C1 | Read/extract exterior and axis analytic constructions, gluing, cone realization and moment solve | Profile algorithms include data, approximation spaces, analyticity/derivative control and stopping criteria |
| C2 | Compute a leading-profile candidate and all required functional margins | An actual source-admissible leading object exists at the stated evidence level, or the precise failing/unresolved inequality is recorded |
| C3 | Extract background recursion, auxiliary phase/support geometry and pulse dynamics | Complete evaluators with derivative and seed/phase accuracy specimens; no missing central inverse operator |
| C4 | Extract mean corrections, residual cycle, final cutoffs and localization/extension | Immutable prefix, interval coverage, bounded tails, full force/reference error records and fixed mathematical identity |
| C5 (deferred) | For any future literal-reproduction proposal, compute new representation-specific feasibility on a nontrivial first interval | Actual spatial/temporal phase bounds, precision/cancellation limits, work and memory estimates fit a specified proposed backend |
| C6 (deferred) | After a future C5 passes, independently integrate from rest and apply the full experiment verifier | First qualified literal-source window; only then begin the source-specific extension ladder |

The current explicit slow-mesh plan is `FeasibilityExcluded` under the declared policy. Literal reproduction is outside the release scope; the broader representation question remains `FeasibilityUnestablished`. The smaller localized onset box is a legitimate source option. The newly extracted slow mesh and pulse conditions remain significant even inside that box; actual label workload, physical phase gradients and cutoff activity decide its numerical reach. Neither the old outer-annulus ratio nor the new asymptotic upper count bounds prove every representation impossible. A fixed cutoff eventually contains the shrinking outer annulus, and any moving-cutoff extension would change the problem. Logarithmic/scaled mathematical representation is allowed. Any proposed alternative full-PDE representation must supply transformed operators, global pressure handling, nonlinear interactions, boundary/axis conditions, and overlap tests. A mechanism ODE or prescribed stress closure receives its own model label.

The compiler may deliver useful mathematical checks before C6. Such artifacts say which identities, inequalities or finite-stage residuals were checked. They do not claim DNS or all-order force flatness. A finite-stage fit cannot establish the full smooth extension demanded by the theorem.

## 6. Resource and release gates

Before every pilot family, compute the full allocation ledger and transform/force counts. The conservative base reservation in the design is approximately 79.68 GiB at `512^3` and 636.71 GiB at `1024^3`, before declared extra scratch and diagnostics. Implementation may reduce this through documented lifetime reuse, but the actual planner must account for every allocation. No claim is based on the memory of one velocity array.

Provide a dry-run CLI preflight that outputs grid/layout, exact target times, method, force coverage, peak planned allocations, evaluation bounds and unsupported capabilities without launching integration. Allocation failures return structured status. A caller requesting a bounded profile with an unbounded provider gets an explicit refusal.

The first public runtime release requires a clean public-only build, passing numerical/transactional/restart fixtures, usable CLI/library examples, and clear evidence status. A concentrating-results release additionally requires qualified benchmark windows. A literal-reproduction claim additionally requires C0–C6. Adapter readiness and a viewer are separate release decisions.

Dependency licensing and public API compatibility are checked against the actual chosen versions at implementation time. The proposed project license is `MIT OR Apache-2.0`; no third-party source is automatically relicensed by this choice. Keep the optional Niva adapter and its private integration references outside the public distribution.

## 7. Deliverables and present completion state

Current user-facing files are the assessment, consolidated specification, construction ledger, implementation plan, source-feasibility extraction, exact v2 manifest, complete review prompt, verification script/results, and checked review packet/bundle. Entry points identify the adopted revision-0.7 engineering baseline and the revision-0.8 review/release decisions. The runtime and case manifest are unchanged; packaging uses explicit per-component revision rules.

The next meaningful implementation result is the independent Python scalar/jet and small-grid arithmetic reference, followed by an honestly labelled PDE diagnostic pilot and the public Rust build with tested Fourier/ETD primitives. The small-grid mpmath DFT reference tests the entire step path at 80 and 120 digits; it does not qualify roundoff in a large concentrating run. The next meaningful scientific result is a from-rest PDE refinement family. Neither is reported as already done. The literal-source gate remains unmet, and that reproduction is not in the current release scope. Optional compiler mathematics and any surrogate decision can proceed separately without delaying runtime implementation.

<!-- END IMPLEMENTATION_PLAN.md -->

---

# Input 6: navier-runtime-verification.py

<!-- BEGIN navier-runtime-verification.py -->

````python
"""Reproduce revision 0.8 checks of the adopted revision 0.7 runtime design.

This is not a Navier--Stokes solver.

Run: python3 navier-runtime-verification.py --output results.json
Requires Python 3 and SymPy. Uses Decimal for independent coefficient references.
"""

from __future__ import annotations

import argparse
from decimal import Decimal, ROUND_CEILING, localcontext
import json
import math
from pathlib import Path
from fractions import Fraction


def phi(z: Decimal, j: int) -> Decimal:
    if z == 0:
        return Decimal(1) / Decimal(math.factorial(j))
    numerator = z.exp() - sum(
        (z**m / Decimal(math.factorial(m)) for m in range(j)), Decimal(0)
    )
    return numerator / z**j


def weights(z: Decimal) -> tuple[Decimal, Decimal, Decimal]:
    p1, p2, p3 = (phi(z, j) for j in (1, 2, 3))
    return p1 - 3*p2 + 4*p3, 2*p2 - 4*p3, -p2 + 4*p3


def scale_checks() -> dict:
    hbar = (-Decimal(11)).exp()
    ln10 = Decimal(10).ln()
    counts = {
        "h_upper_relaxation_Md_0": str(hbar),
        "minus_log10_Q_at_epsilon_quarter": str(Decimal(4).ln()/hbar/ln10),
        "minus_log10_tau_at_normalized_growth_10": str(1/hbar),
        "first_dyadic_band_at_epsilon_quarter_relaxed_h": int(
            (2/hbar).to_integral_value(rounding=ROUND_CEILING)
        ),
        "log10_annulus_radius_ratio_lower_relaxation": str(
            (Decimal(3)+Decimal(110).ln()+110-Decimal(4).ln())/(2*ln10)
        ),
    }
    carriers = []
    for eps in map(Decimal, ("0.99", "0.5", "0.25", "0.249", "0.01")):
        carrier = int((1/eps.sqrt()).to_integral_value(rounding=ROUND_CEILING))
        carriers.append({"epsilon": str(eps), "carrier": carrier})
    assert [item["carrier"] for item in carriers] == [2, 2, 2, 3, 10]
    # A short significand can store an extremely small distance when exponent
    # storage is separate. Forming the difference 1-tau is a different operation.
    tiny = Decimal(2)**-120000
    assert tiny != 0 and float(tiny) == 0
    assert 1-tiny == 1  # This Decimal context, too, loses the subtraction.
    growth = (hbar * Decimal(-120000) * Decimal(2).ln()).exp()
    counts.update({"ceiling_examples": carriers,
                   "scaled_time_example": {"mantissa": 1, "binary_exponent": -120000,
                                            "Q_to_h": str(growth)},
                   "interpretation": "Relaxed parameter bounds, not an admissible source instance."})
    return counts


def coefficient_checks() -> dict:
    examples = []
    for zs in ("0", "-0.5", "-2", "-10", "-50", "-100", "-10000"):
        z = Decimal(zs)
        w1, w2, w3 = weights(z)
        assert abs(w1+2*w2+w3-phi(z, 1)) < Decimal("1e-65")
        if z <= -50:
            good = (-1/z**2-4/z**3, 2/z**2+4/z**3, -1/z-3/z**2-4/z**3)
            bad = (-1/z**2-4/z**3, 2/z**2+8/z**3, -1/(2*z)-3/z**2-4/z**3)
            corrected_errors = [float(abs((a-b)/b)) for a,b in zip(good,(w1,w2,w3))]
            assert max(corrected_errors) < 1e-18
            examples.append({"z": zs,
                             "corrected_asymptotic_relative_errors": corrected_errors,
                             "rev05_asymptotic_relative_errors": [float(abs((a-b)/b)) for a,b in zip(bad,(w1,w2,w3))]})
    return {"constant_source_identity": "w1 + 2*w2 + w3 = phi1 passed",
            "large_negative_examples": examples}


def source_screening_checks() -> dict:
    """Evaluate a sufficient proof screen, never an admitted source instance."""
    h = (-Decimal(11)).exp()
    ln2 = Decimal(2).ln()
    ln10 = Decimal(10).ln()
    quarter_band = int((2/h).to_integral_value(rounding=ROUND_CEILING))

    def row(ell: int) -> dict:
        eps = (-h*ell*ln2).exp()
        carrier = int((1/eps.sqrt()).to_integral_value(rounding=ROUND_CEILING))
        slow = ell**2
        test = Decimal(slow**2)*(eps + eps**2 + Decimal(1)/carrier)
        envelope = Decimal(slow**2)*(eps + eps**2 + eps.sqrt())
        return {"ell": ell, "epsilon": str(eps), "carrier": carrier,
                "S_star": slow, "S_star_cubed": str(slow**3),
                "S_star_ninth_power": str(slow**9),
                "chart_mesh_S_star_minus_3": str(Decimal(1)/slow**3),
                "explicit_proof_test": str(test),
                "smooth_sufficient_envelope": str(envelope),
                "minus_log10_Q": str(ell*ln2/ln10),
                "auxiliary_traversal_chart_proxy_epsilon_S": str(eps*slow),
                "auxiliary_gaussian_chart_proxy_epsilon_sqrtS": str(eps*ell)}

    # Each term of ell^4*(eps+eps^2+sqrt(eps)) decreases after this point.
    monotone_start = int((8/(h*ln2)).to_integral_value(rounding=ROUND_CEILING))
    lower = max(quarter_band, monotone_start)
    assert Decimal(row(lower)["smooth_sufficient_envelope"]) > 1
    upper = lower
    for _ in range(32):
        upper *= 2
        if Decimal(row(upper)["smooth_sufficient_envelope"]) <= 1:
            break
    else:
        raise ArithmeticError("No bracket for sufficient smallness screen")
    for _ in range(64):
        if upper-lower == 1:
            break
        middle = (lower+upper)//2
        if Decimal(row(middle)["smooth_sufficient_envelope"]) <= 1:
            upper = middle
        else:
            lower = middle
    assert upper-lower == 1
    assert Decimal(row(upper-1)["smooth_sufficient_envelope"]) > 1
    assert Decimal(row(upper)["smooth_sufficient_envelope"]) <= 1
    assert Decimal(row(quarter_band)["explicit_proof_test"]) > 1

    sqrt2 = Decimal(2).sqrt()
    lam, temporal = 4-sqrt2, 4+sqrt2
    rho = lam.ln()/temporal.ln()
    dr = 2*((1+h)*rho-h*Decimal("0.00001"))
    maps = []
    for ell in (quarter_band, upper):
        slow = Decimal(ell**2)
        covering = int(((1+h)*ell*ln2-slow.ln())//temporal.ln())
        ci = (covering*temporal.ln()-(1+h)*ell*ln2).exp()
        mi = (covering*lam.ln()-dr*ell*ln2/2).exp()
        assert 1/temporal < ci*slow <= 1
        maps.append({"ell": ell, "covering_index": covering,
                     "c_i": str(ci), "M_i": str(mi)})
    return {"h_is_relaxed_boundary_not_admissible": str(h),
            "quarter_screen": row(quarter_band),
            "sufficient_screen_predecessor": row(upper-1),
            "sufficient_screen_first_band_in_monotone_range": row(upper),
            "auxiliary_map": {"rho_g": str(rho), "d_r": str(dr),
                              "rows": maps},
            "q_star": None, "source_profiles_computed": False,
            "interpretation": "One displayed sufficient test from Lemma 7.1. Not a necessary condition for every realization; not the complete q_star gate. S powers are mesh scale/count factors, not measured pulse counts or universal work lower bounds."}


def source_mesh_review_checks(screen: dict) -> dict:
    """Check review arithmetic and a declared explicit-mesh resource policy.

    The policy deliberately resolves every slow-mesh interval on a prescribed
    chart segment. It is not a lower bound on every independent PDE method.
    Amplitude factors omit the source's logarithmic/profile/weight factors.
    """
    h = Decimal(screen["h_is_relaxed_boundary_not_admissible"])
    ln2 = Decimal(2).ln()
    illustrative_lambda = Decimal(128)
    radius = (8/illustrative_lambda).sqrt()
    segment = Decimal(1)/4
    max_intervals = 1024
    assert radius == segment
    rows = []
    for source in (screen["quarter_screen"],
                   screen["sufficient_screen_first_band_in_monotone_range"]):
        ell = source["ell"]
        eps = Decimal(source["epsilon"])
        carrier = source["carrier"]
        mesh_denominator = ell**6
        mesh = Decimal(1)/mesh_denominator
        required_intervals = (mesh_denominator+3)//4
        assert 4*required_intervals >= mesh_denominator
        assert 4*(required_intervals-1) < mesh_denominator
        assert mesh == Decimal(source["chart_mesh_S_star_minus_3"])
        assert eps.sqrt() <= Decimal(1)/2
        assert required_intervals > max_intervals
        traversal = eps*ell**2
        gaussian = eps*ell
        rows.append({
            "ell": ell,
            "sqrt_epsilon_pure_asymptotic_factor": str(eps.sqrt()),
            "chart_mesh_spacing": str(mesh),
            "slow_support_full_extent_upper_bound": str(2*mesh),
            "carrier_over_ell6": str(Decimal(carrier)/mesh_denominator),
            "mesh_over_illustrative_core_radius": str(mesh/radius),
            "auxiliary_traversal_chart_proxy": str(traversal),
            "auxiliary_gaussian_chart_proxy": str(gaussian),
            "auxiliary_traversal_proxy_over_slow_full_extent_bound": str(traversal/(2*mesh)),
            "auxiliary_gaussian_proxy_over_slow_full_extent_bound": str(gaussian/(2*mesh)),
            "policy_required_intervals_per_axis": str(required_intervals),
            "policy_exceeds_cap": True,
        })

    # Check the review's proposed carrier/mesh balance as a heuristic only.
    # Its logarithmic proxy is strictly increasing beyond 12/(h log 2).
    def balance(ell: int) -> Decimal:
        return h*ell*ln2/2-6*Decimal(ell).ln()

    lower = int((12/(h*ln2)).to_integral_value(rounding=ROUND_CEILING))
    assert balance(lower) < 0
    upper = lower
    for _ in range(32):
        upper *= 2
        if balance(upper) >= 0:
            break
    else:
        raise ArithmeticError("No bracket for heuristic carrier/mesh balance")
    for _ in range(64):
        if upper-lower == 1:
            break
        middle = (lower+upper)//2
        if balance(middle) >= 0:
            upper = middle
        else:
            lower = middle
    assert upper-lower == 1 and balance(lower) < 0 <= balance(upper)
    crossings = []
    for ell in (lower, upper):
        carrier = int((h*ell*ln2/2).exp().to_integral_value(rounding=ROUND_CEILING))
        crossings.append({"ell": ell, "carrier": str(carrier),
                          "ell6": str(ell**6),
                          "carrier_over_ell6": str(Decimal(carrier)/ell**6)})
    assert int(crossings[0]["carrier"]) < lower**6
    assert int(crossings[1]["carrier"]) >= upper**6
    return {
        "status": "passed",
        "arithmetic_precision_decimal_digits": 75,
        "illustrative_core": {"Lambda": "128", "chart_radius": "1/4",
                              "source_admissibility_claimed": False},
        "rows": rows,
        "amplitude_scope": "Pure q^(h/2) at q=Q only; omitted constants, logarithms, profiles and support weights prevent an absolute pulse amplitude bound. No pulse omission is authorized.",
        "time_scope": "The full slow cutoff extent is at most 2 Q ell^-6 in physical time. Q^(1+h)L_s is an auxiliary-rectangle traversal duration. Actual support is their intersection with all other cutoffs; epsilon*S and epsilon*sqrt(S) are proxies with unknown constants.",
        "heuristic_carrier_mesh_crossing": {
            "classification": "Scale balance only; neither a necessary admissibility test nor the source q_star",
            "predecessor": crossings[0], "first_passing_in_monotone_range": crossings[1]},
        "explicit_mesh_policy": {
            "policy_id": "explicit-slow-mesh-v1",
            "chart_segment_length": "1/4",
            "required_spacing_at_most": "ell^-6",
            "maximum_stored_intervals_per_axis": max_intervals,
            "cap_origin": "Declared policy for this screen, not measured hardware capacity or a universal resource bound",
            "minimum_tested_band": screen["quarter_screen"]["ell"],
            "band_scope": "Both displayed relaxed bands and every larger integer under this same policy; required intervals increase as ell^6",
            "scope": "Explicit subdivision of every mesh interval across the declared chart segment, conditional on undertaking that mesh-resolving plan; actual localized active support has not been admitted",
            "status": "FeasibilityExcluded",
            "reason": "ceil((1/4)*ell^6) exceeds the declared 1024-interval cap in each tested row",
            "universal_representation_exclusion_claimed": False,
        },
        "literal_target_status": "FeasibilityUnestablished",
        "direct_reproduction_in_current_release": False,
        "compiler_scope": "MathematicsVerificationOnly",
        "averaged_stress_surrogate": "SeparateDecisionDeferred",
    }


def float_phi(z: float, j: int) -> float:
    if abs(z) <= 1:
        value = 1/math.factorial(18+j)
        for m in range(17, -1, -1):
            value = value*z + 1/math.factorial(m+j)
        return value
    value = math.expm1(z)/z
    for index in range(1, j):
        value = (value-1/math.factorial(index))/z
    return value


def float_weights(z: float) -> tuple[float, float, float]:
    if abs(z) <= 1:
        a, b, c = (float_phi(z, j) for j in (1, 2, 3))
        return a-3*b+4*c, 2*b-4*c, -b+4*c
    if z <= -50:
        r = 1/z
        return -r*r*(1+4*r), 2*r*r*(1+2*r), -r*(1+3*r+4*r*r)
    e = math.exp(z)
    return ((e*(z*z-3*z+4)-z-4)/z**3,
            2*(e*(z-2)+z+2)/z**3,
            (e*(4-z)-z*z-3*z-4)/z**3)


def floating_junction_checks() -> dict:
    rows = []
    values = [math.nextafter(-50.0, -math.inf), -50.0,
              math.nextafter(-50.0, math.inf),
              math.nextafter(-1.0, -math.inf), -1.0,
              math.nextafter(-1.0, math.inf)]
    for z in values:
        exact = weights(Decimal.from_float(z))
        observed = float_weights(z)
        errors = [float(abs(Decimal.from_float(a)-b)) for a, b in zip(observed, exact)]
        conditioning_scale = sum(abs(float(phi(Decimal.from_float(z), j))) for j in (1,2,3))
        assert max(errors) <= 256*math.ulp(1.0)*conditioning_scale
        for argument in (z, z/2):
            for j in (1,2,3):
                reference = phi(Decimal.from_float(argument), j)
                error = abs(Decimal.from_float(float_phi(argument,j))-reference)
                assert error <= Decimal(64)*Decimal.from_float(math.ulp(1.0))*abs(reference)
        rows.append({"z": z, "weights": observed,
                     "absolute_errors_against_75_digit_reference": errors})
    return {"status": "passed", "rows": rows,
            "scope": "Executed Python binary64 branch implementations, including half arguments for HO. Not Rust or a global floating-point proof."}


def benchmark_geometry_checks() -> dict:
    params = [("similarity-mms-v1", Decimal(1)/64, Decimal(1)/4, Decimal(3)/8),
              ("similarity-mms-v2", Decimal(1)/128, Decimal(3)/10, Decimal(21)/50)]
    rows = []
    for name, target, inner, outer in params:
        edges = []
        for radius in (inner, outer):
            x = radius**2/(2*target)
            ratio = (2*x).sqrt()*(Decimal("0.5")-x).exp()
            edges.append({"radius": str(radius), "X_at_t0_z0": str(x),
                          "uncut_swirl_amplitude_relative_to_peak": str(ratio)})
        endpoints = []
        for index in range(1,7):
            tau = target/2**index
            eta = Decimal(1)/2
            q = tau/(1-eta**2)
            # The largest spherical radius for X<=8, |eta|<=1/2 is at the edge.
            max_radius_squared = 16*q + eta**2*q**(Decimal(3)/4)
            endpoints.append({"k": index, "remaining": str(tau),
                              "full_nominal_annulus_radius_squared_max": str(max_radius_squared),
                              "nominal_annulus_inside_uncut_ball": max_radius_squared<=inner**2,
                              "equatorial_peak_radius_cells_at_N512": str(512*tau.sqrt())})
        rows.append({"case": name, "target": str(target), "edges": edges,
                     "endpoints": endpoints})
    assert all(r["nominal_annulus_inside_uncut_ball"] for r in rows[1]["endpoints"])
    ceilings = []
    for n in (128,256,512,1024):
        accepted = [k for k in range(1,32) if Decimal(n)*(Decimal(1)/128/2**k).sqrt() >= 12]
        ceilings.append({"N":n, "last_endpoint_under_radius_12_cell_screen":max(accepted,default=0)})
    assert ceilings[2]["last_endpoint_under_radius_12_cell_screen"] == 3
    return {"cases": rows, "v2_radius_screen":ceilings,
            "diagnostic_eta_limit":"1/2",
            "scope":"Analytic geometry and amplitude ratios. At t=0 the ramp makes actual velocity zero. Edge amplitude is not removed energy or an error in the localized solution. Cell counts are screening conventions, not convergence results."}


def clock_checks() -> dict:
    elapsed, remaining, dt = 1.0-2.0**-53, 2.0**-53, 2.0**-54
    e1, r1 = elapsed+dt, remaining-dt
    assert e1+r1 == 1.0
    assert Fraction.from_float(e1)+Fraction.from_float(r1) != 1
    target_ticks = 1 << 100
    elapsed_ticks, remaining_ticks, step_ticks = target_ticks-8, 8, 4
    elapsed_ticks += step_ticks
    remaining_ticks -= step_ticks
    assert elapsed_ticks+remaining_ticks == target_ticks
    assert 0 < remaining_ticks < target_ticks < 2**128
    assert step_ticks%4 == 0
    return {"binary64_dyadic_counterexample": {
                "floating_sum_equals_target":True,
                "exact_sum_error":str(Fraction.from_float(e1)+Fraction.from_float(r1)-1)},
            "u128_tick_invariant": "passed",
            "scope":"Clock arithmetic fixture; no Rust state transaction was executed."}


def symbolic_checks() -> dict:
    import sympy as s

    z = s.symbols("z", nonzero=True)
    p = [(s.exp(z)-sum(z**m/s.factorial(m) for m in range(j)))/z**j for j in (1,2,3)]
    w = [p[0]-3*p[1]+4*p[2], 2*p[1]-4*p[2], -p[1]+4*p[2]]
    closed = [(s.exp(z)*(z*z-3*z+4)-z-4)/z**3,
              2*(s.exp(z)*(z-2)+z+2)/z**3,
              (s.exp(z)*(4-z)-z*z-3*z-4)/z**3]
    assert all(s.simplify(a-b) == 0 for a,b in zip(w,closed))

    nu, lam = s.symbols("nu lam", positive=True)
    scalings = [(nu,1,nu,nu**2),
                (s.sqrt(nu),1/s.sqrt(nu),1,nu),
                (lam*s.sqrt(nu),lam/s.sqrt(nu),lam**2,lam**2*nu)]
    for alpha,beta,gamma,pressure in scalings:
        factors = [alpha*gamma,alpha**2*beta,nu*alpha*beta**2,pressure*beta]
        assert all(s.simplify(f-factors[0]) == 0 for f in factors)
    assert s.simplify(lam**2*(1-(1-lam**-2))) == 1

    X, eta, h = s.symbols("X eta h", real=True)
    a = eta+s.Rational(1,32)
    M = a*X*s.exp(-X)
    U = s.diff(M,X)
    D, L, d = s.Rational(1,2)-h, 1-2*h*eta**2, 1-eta**2
    V0 = (2*eta*X*U-2*D*eta*M-d*s.diff(M,eta))/L
    derivative_of_streamfunction = (2*D*eta*M-2*eta*X*U+d*s.diff(M,eta))/L
    assert s.simplify(V0+derivative_of_streamfunction) == 0
    assert s.limit(M,X,0) == 0 and s.limit(M,X,s.oo) == 0
    assert s.simplify(s.diff(-s.exp(-2*X)/32,X)-s.exp(-2*X)/16) == 0

    # Curl of a smooth Cartesian vector potential, with a radial swirl added.
    # Q(r^2,z,t) includes all physical cutoffs and time factors; no axis division.
    x,y,zz,t = s.symbols("x y zz t", real=True)
    R2=s.symbols("R2", real=True)
    G=s.Function("G")(R2,zz,t).subs(R2,x*x+y*y)
    B=s.Function("B")(R2,zz,t).subs(R2,x*x+y*y)
    vel=[-x*s.diff(G,zz)-y*B, -y*s.diff(G,zz)+x*B,
         2*G+x*s.diff(G,x)+y*s.diff(G,y)]
    assert s.simplify(sum(s.diff(v,c) for v,c in zip(vel,(x,y,zz)))) == 0

    # Independently check the zero-linear-operator tableau of HO(5.19).
    half=s.Rational(1,2)
    ph1,ph2,ph3=s.Integer(1),half,s.Rational(1,6)
    mat=s.zeros(5,5)
    mat[1,0]=half*ph1
    mat[2,0]=half*ph1-ph2
    mat[2,1]=ph2
    mat[3,0]=ph1-2*ph2
    mat[3,1]=mat[3,2]=ph2
    a52=half*ph2-ph3+ph2/4-ph3/2
    a54=ph2/4-a52
    mat[4,0]=half*ph1-2*a52-a54
    mat[4,1]=mat[4,2]=a52
    mat[4,3]=a54
    c=s.Matrix([0,half,half,1,half])
    b=s.Matrix([s.Rational(1,6),0,0,s.Rational(1,6),s.Rational(2,3)])
    assert mat*s.ones(5,1) == c
    conditions=[sum(b),b.dot(c),b.dot(c.applyfunc(lambda v:v**2)),b.dot(mat*c),
                b.dot(c.applyfunc(lambda v:v**3)),b.dot(s.diag(*c)*mat*c),
                b.dot(mat*c.applyfunc(lambda v:v**2)),b.dot(mat*mat*c)]
    expected=[1,half,s.Rational(1,3),s.Rational(1,6),s.Rational(1,4),
              s.Rational(1,8),s.Rational(1,12),s.Rational(1,24)]
    assert conditions == expected, (conditions,expected)

    # Nonzero-operator identities exercise every phi-dependent HO row.
    def symbolic_phi(argument, j):
        return (s.exp(argument)-sum(argument**m/s.factorial(m) for m in range(j)))/argument**j
    full=[symbolic_phi(z,j) for j in (1,2,3)]
    at_half=[symbolic_phi(z/2,j) for j in (1,2,3)]
    nonzero=s.zeros(5,5)
    nonzero[1,0]=half*at_half[0]
    nonzero[2,0]=half*at_half[0]-at_half[1]
    nonzero[2,1]=at_half[1]
    nonzero[3,0]=full[0]-2*full[1]
    nonzero[3,1]=nonzero[3,2]=full[1]
    q52=half*at_half[1]-full[2]+full[1]/4-at_half[2]/2
    q54=at_half[1]/4-q52
    nonzero[4,0]=half*at_half[0]-2*q52-q54
    nonzero[4,1]=nonzero[4,2]=q52
    nonzero[4,3]=q54
    for i in range(1,5):
        assert s.simplify(sum(nonzero[i,j] for j in range(i))-c[i]*symbolic_phi(c[i]*z,1)) == 0
    for i in (3,4):
        assert s.simplify(sum(nonzero[i,j]*c[j] for j in range(i))-c[i]**2*symbolic_phi(c[i]*z,2)) == 0
    b1=full[0]-3*full[1]+4*full[2]
    b4=-full[1]+4*full[2]
    b5=4*full[1]-8*full[2]
    assert s.simplify(b1+b4+b5-full[0]) == 0

    return {"ETD_closed_forms": "passed", "viscosity_and_periodic_scalings": "passed",
            "streamfunction_and_zero_U_moment": "passed", "radial_pressure_identity": "passed",
            "localized_Cartesian_divergence": "identically zero symbolically",
            "HO_zero_operator_order_four_conditions": "all eight conditions passed",
            "HO_nonzero_operator_rows": "four row sums, row-4/5 first moments, and output sum passed symbolically",
            "scope": "Identities for the specified manufactured profile, not Theorem 4.6 admissibility."}


def memory_checks() -> list[dict]:
    rows=[]
    for n in (128,256,512,1024):
        m=3*n//2
        h=n*n*(n//2+1)
        hp=m*m*(m//2+1)
        state=48*h
        real_vector=24*m**3
        padded_spectrum=48*hp
        # Conservative reservation: 12 retained vectors, 3 padded real vectors,
        # one padded vector spectrum, 6 retained scalar coefficient tables.
        total=12*state+3*real_vector+padded_spectrum+6*8*h
        assert state < 48*n**3
        rows.append({"N":n,"retained_vector_GiB":state/2**30,
                     "padded_real_vector_GiB":real_vector/2**30,
                     "reserved_base_GiB":total/2**30,
                     "excluded": "FFT/provider scratch, planner storage, I/O, diagnostic grids, allocator overhead"})
    return rows


def main() -> None:
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--output",type=Path)
    args=parser.parse_args()
    with localcontext() as context:
        context.prec=75
        screen = source_screening_checks()
        result={"status":"passed","revision":"0.8","runtime_design_revision":"0.7","scales":scale_checks(),
                "coefficients":coefficient_checks(),"symbolic":symbolic_checks(),
                "source_screening":screen,
                "source_mesh_review":source_mesh_review_checks(screen),
                "floating_junctions":floating_junction_checks(),
                "benchmark_geometry":benchmark_geometry_checks(),
                "clock":clock_checks(),
                "memory":memory_checks(),
                "not_performed":["Rust compilation","PDE integration","source instance admissibility","formal proof build"]}
    text=json.dumps(result,indent=2)+"\n"
    if args.output:
        args.output.write_text(text,encoding="utf-8")
    print(text,end="")


if __name__ == "__main__":
    main()
````

<!-- END navier-runtime-verification.py -->

---

# Input 7: navier-runtime-verification-results.json

<!-- BEGIN navier-runtime-verification-results.json -->

````json
{
  "status": "passed",
  "revision": "0.8",
  "runtime_design_revision": "0.7",
  "scales": {
    "h_upper_relaxation_Md_0": "0.0000167017007902456593126355173605808790779380469592871244781247371135239542313",
    "minus_log10_Q_at_epsilon_quarter": "36047.8252418211897841433395471114161075828546481498598175851440222499803664",
    "minus_log10_tau_at_normalized_growth_10": "59874.1417151978184553264857922577816142610796957409686527715143799324211158",
    "first_dyadic_band_at_epsilon_quarter_relaxed_h": 119749,
    "log10_annulus_radius_ratio_lower_relaxation": "25.2573045744488595874501450096852062437185920416782789914544807970841210072",
    "ceiling_examples": [
      {
        "epsilon": "0.99",
        "carrier": 2
      },
      {
        "epsilon": "0.5",
        "carrier": 2
      },
      {
        "epsilon": "0.25",
        "carrier": 2
      },
      {
        "epsilon": "0.249",
        "carrier": 3
      },
      {
        "epsilon": "0.01",
        "carrier": 10
      }
    ],
    "scaled_time_example": {
      "mantissa": 1,
      "binary_exponent": -120000,
      "Q_to_h": "0.249272546315873252378435942885191419201259681991126382489195064908686105291"
    },
    "interpretation": "Relaxed parameter bounds, not an admissible source instance."
  },
  "coefficients": {
    "constant_source_identity": "w1 + 2*w2 + w3 = phi1 passed",
    "large_negative_examples": [
      {
        "z": "-50",
        "corrected_asymptotic_relative_errors": [
          1.1128048035861386e-20,
          2.0894790019609108e-22,
          4.424489880630908e-24
        ],
        "rev05_asymptotic_relative_errors": [
          1.1128048035861386e-20,
          0.041666666666666664,
          0.5310110450297366
        ]
      },
      {
        "z": "-100",
        "corrected_asymptotic_relative_errors": [
          3.9928815475956974e-42,
          3.871915811776788e-44,
          3.9868909883158174e-46
        ],
        "rev05_asymptotic_relative_errors": [
          3.9928815475956974e-42,
          0.02040816326530612,
          0.5152514427040396
        ]
      },
      {
        "z": "-10000",
        "corrected_asymptotic_relative_errors": [
          0.0,
          0.0,
          0.0
        ],
        "rev05_asymptotic_relative_errors": [
          0.0,
          0.00020004000800160032,
          0.5001500250014994
        ]
      }
    ]
  },
  "symbolic": {
    "ETD_closed_forms": "passed",
    "viscosity_and_periodic_scalings": "passed",
    "streamfunction_and_zero_U_moment": "passed",
    "radial_pressure_identity": "passed",
    "localized_Cartesian_divergence": "identically zero symbolically",
    "HO_zero_operator_order_four_conditions": "all eight conditions passed",
    "HO_nonzero_operator_rows": "four row sums, row-4/5 first moments, and output sum passed symbolically",
    "scope": "Identities for the specified manufactured profile, not Theorem 4.6 admissibility."
  },
  "source_screening": {
    "h_is_relaxed_boundary_not_admissible": "0.0000167017007902456593126355173605808790779380469592871244781247371135239542313",
    "quarter_screen": {
      "ell": 119749,
      "epsilon": "0.249997926124172436356372024208497704318130774786992803615380110784457427393",
      "carrier": 3,
      "S_star": 14339823001,
      "S_star_cubed": "2948705313461059605824006469001",
      "S_star_ninth_power": "25638588803501564852542537344628179198193452325960586168238263776934491496938138724902407001",
      "chart_mesh_S_star_minus_3": "3.39131887962803689435180465159829424524821724868525252196518455047739120995E-31",
      "explicit_proof_test": "132802406878881226676.530226842373047728241112094080845082243407242360690214",
      "smooth_sufficient_envelope": "167073734375825744455.322617130579433716742924436825981886643743308309809186",
      "minus_log10_Q": "36048.0409507660841456500189043633154624639701152060357133823780425100785574",
      "auxiliary_traversal_chart_proxy_epsilon_S": "3584926011.23770668495331218565794420003682870561647028090530366998489077064",
      "auxiliary_gaussian_chart_proxy_epsilon_sqrtS": "29937.0016554435250812391935269433915943918421499676012401381528863279924729"
    },
    "sufficient_screen_predecessor": {
      "ell": 11217669,
      "epsilon": "3.98823440830123272609054808093137691132915249225028783296444666289951095711E-57",
      "carrier": 15834693518268427388612454452,
      "S_star": 125836097793561,
      "S_star_cubed": "1992579815875443536936199606813296575817481",
      "S_star_ninth_power": "7911287736751075901466519111662857407846209702648836185994495581847374760943182586941665869146924766688541811245951582529095641",
      "chart_mesh_S_star_minus_3": "5.01861954052088094594558314456537740463670825371837703729670545550708715247E-43",
      "explicit_proof_test": "1.00000189391996665960229767034864291075979634480526822276566150210091864944",
      "smooth_sufficient_envelope": "1.00000189391996665960229767039584860593262207795502192498051952125510296407",
      "minus_log10_Q": "3376854.85042997627013210717344520896709369381939116965849320150743426659447",
      "auxiliary_traversal_chart_proxy_epsilon_S": "5.01863855026638811825839182444386021009560025073872548929464570839048420309E-43",
      "auxiliary_gaussian_chart_proxy_epsilon_sqrtS": "4.47386934867340810132514324004733979055327827085887940649224514325612941787E-50"
    },
    "sufficient_screen_first_band_in_monotone_range": {
      "ell": 11217670,
      "epsilon": "3.98818823782839023925340124579505695281751449520198453692087409848684616864E-57",
      "carrier": 15834785175573390502848019699,
      "S_star": 125836120228900,
      "S_star_cubed": "1992580881647803167380206290948863569000000",
      "S_star_ninth_power": "7911300431303497191886727179771076709621565035978804898920650620623884169636757958063098749634517988368449009000000000000000000",
      "chart_mesh_S_star_minus_3": "5.01861685621027701945479483312211290192633848273357532393670529210182518260E-43",
      "explicit_proof_test": "0.999996462136328749665188922777446335044707021120921514896279147854225936973",
      "smooth_sufficient_envelope": "0.999996462136328749665188922787728159216896068075892391140008545476197868801",
      "minus_log10_Q": "3376855.15145997193411330238718410369158672058758105112060174281786172772157",
      "auxiliary_traversal_chart_proxy_epsilon_S": "5.01858134590858141192438235339119827284067628619905418377853804063270674042E-43",
      "auxiliary_gaussian_chart_proxy_epsilon_sqrtS": "4.47381795498403983351657015529178365279124478273924458802811817483729396606E-50"
    },
    "auxiliary_map": {
      "rho_g": "0.562471372415831813385720078553123555629093266027151411003879772992366802725",
      "d_r": "1.12496153295475994708400068333944451130304807518339561227302272367716322315",
      "rows": [
        {
          "ell": 119749,
          "covering_index": 49129,
          "c_i": "1.64735928651914227734536440611190266758366596045783260074417233710323018225E-11",
          "M_i": "8.60514420621827812506541415511162426834877208752068569080994330831835937388E-7"
        },
        {
          "ell": 11217670,
          "covering_index": 4603591,
          "c_i": "2.15766985430665502793388932992475424006880341033958570419637600928912489510E-15",
          "M_i": "5.64092601800131028100179754040143815424340804075442212592347091480150286608E-9"
        }
      ]
    },
    "q_star": null,
    "source_profiles_computed": false,
    "interpretation": "One displayed sufficient test from Lemma 7.1. Not a necessary condition for every realization; not the complete q_star gate. S powers are mesh scale/count factors, not measured pulse counts or universal work lower bounds."
  },
  "source_mesh_review": {
    "status": "passed",
    "arithmetic_precision_decimal_digits": 75,
    "illustrative_core": {
      "Lambda": "128",
      "chart_radius": "1/4",
      "source_admissibility_claimed": false
    },
    "rows": [
      {
        "ell": 119749,
        "sqrt_epsilon_pure_asymptotic_factor": "0.499997926119871457568808853287950401416600628944563018721131691979822674458",
        "chart_mesh_spacing": "3.39131887962803689435180465159829424524821724868525252196518455047739120995E-31",
        "slow_support_full_extent_upper_bound": "6.78263775925607378870360930319658849049643449737050504393036910095478241990E-31",
        "carrier_over_ell6": "1.01739566388841106830554139547948827357446517460557575658955536514321736298E-30",
        "mesh_over_illustrative_core_radius": "1.35652755185121475774072186063931769809928689947410100878607382019095648398E-30",
        "auxiliary_traversal_chart_proxy": "3584926011.23770668495331218565794420003682870561647028090530366998489077064",
        "auxiliary_gaussian_chart_proxy": "29937.0016554435250812391935269433915943918421499676012401381528863279924729",
        "auxiliary_traversal_proxy_over_slow_full_extent_bound": "5285445188850693990753466814108738977102.29096427342422016664864332209468536",
        "auxiliary_gaussian_proxy_over_slow_full_extent_bound": "44137697925249429980655093688538016.8277170662324814755878266093522459033925",
        "policy_required_intervals_per_axis": "737176328365264901456001617251",
        "policy_exceeds_cap": true
      },
      {
        "ell": 11217670,
        "sqrt_epsilon_pure_asymptotic_factor": "6.31521039857611572137854393727287575481404936883605345946090917287410564370E-29",
        "chart_mesh_spacing": "5.01861685621027701945479483312211290192633848273357532393670529210182518260E-43",
        "slow_support_full_extent_upper_bound": "1.00372337124205540389095896662442258038526769654671506478734105842036503652E-42",
        "carrier_over_ell6": "7.94687197966012284730899303197326463185149803510488014171908767652647492989E-15",
        "mesh_over_illustrative_core_radius": "2.00744674248411080778191793324884516077053539309343012957468211684073007304E-42",
        "auxiliary_traversal_chart_proxy": "5.01858134590858141192438235339119827284067628619905418377853804063270674042E-43",
        "auxiliary_gaussian_chart_proxy": "4.47381795498403983351657015529178365279124478273924458802811817483729396606E-50",
        "auxiliary_traversal_proxy_over_slow_full_extent_bound": "0.499996462142586989343365371278804553508732791971570236171613693438661501281",
        "auxiliary_gaussian_proxy_over_slow_full_extent_bound": "4.45722206253693493696431942889035382132593303218556292145885637069606701999E-8",
        "policy_required_intervals_per_axis": "498145220411950791845051572737215892250000",
        "policy_exceeds_cap": true
      }
    ],
    "amplitude_scope": "Pure q^(h/2) at q=Q only; omitted constants, logarithms, profiles and support weights prevent an absolute pulse amplitude bound. No pulse omission is authorized.",
    "time_scope": "The full slow cutoff extent is at most 2 Q ell^-6 in physical time. Q^(1+h)L_s is an auxiliary-rectangle traversal duration. Actual support is their intersection with all other cutoffs; epsilon*S and epsilon*sqrt(S) are proxies with unknown constants.",
    "heuristic_carrier_mesh_crossing": {
      "classification": "Scale balance only; neither a necessary admissibility test nor the source q_star",
      "predecessor": {
        "ell": 17274000,
        "carrier": "26567852993651524917548414624612050212392222",
        "ell6": "26567916127874706387686976000000000000000000",
        "carrier_over_ell6": "0.999997623666723517297384113243397408860203123461600717189046755472987251133"
      },
      "first_passing_in_monotone_range": {
        "ell": 17274001,
        "carrier": "26568006778617507675334759249850259116073994",
        "ell6": "26567925356051147147660466464052346243644001",
        "carrier_over_ell6": "1.00000306469418553913389452929381473196882091461477101607882193929587791508"
      }
    },
    "explicit_mesh_policy": {
      "policy_id": "explicit-slow-mesh-v1",
      "chart_segment_length": "1/4",
      "required_spacing_at_most": "ell^-6",
      "maximum_stored_intervals_per_axis": 1024,
      "cap_origin": "Declared policy for this screen, not measured hardware capacity or a universal resource bound",
      "minimum_tested_band": 119749,
      "band_scope": "Both displayed relaxed bands and every larger integer under this same policy; required intervals increase as ell^6",
      "scope": "Explicit subdivision of every mesh interval across the declared chart segment, conditional on undertaking that mesh-resolving plan; actual localized active support has not been admitted",
      "status": "FeasibilityExcluded",
      "reason": "ceil((1/4)*ell^6) exceeds the declared 1024-interval cap in each tested row",
      "universal_representation_exclusion_claimed": false
    },
    "literal_target_status": "FeasibilityUnestablished",
    "direct_reproduction_in_current_release": false,
    "compiler_scope": "MathematicsVerificationOnly",
    "averaged_stress_surrogate": "SeparateDecisionDeferred"
  },
  "floating_junctions": {
    "status": "passed",
    "rows": [
      {
        "z": -50.00000000000001,
        "weights": [
          -0.0003679999999999999,
          0.0007679999999999997,
          0.018831999999999998
        ],
        "absolute_errors_against_75_digit_reference": [
          8.091931078193404e-21,
          9.207289879444472e-20,
          5.614265441986392e-19
        ]
      },
      {
        "z": -50.0,
        "weights": [
          -0.00036800000000000005,
          0.000768,
          0.018832
        ],
        "absolute_errors_against_75_digit_reference": [
          5.449397759961018e-20,
          1.9456498034568516e-20,
          1.5161206457501072e-18
        ]
      },
      {
        "z": -49.99999999999999,
        "weights": [
          -0.0003680000000000001,
          0.0007680000000000002,
          0.018832
        ],
        "absolute_errors_against_75_digit_reference": [
          8.659669028863269e-21,
          2.2565677615031225e-20,
          9.986322046520397e-19
        ]
      },
      {
        "z": -1.0000000000000002,
        "weights": [
          0.05696447062846129,
          0.20727664702865298,
          0.16060279414278822
        ],
        "absolute_errors_against_75_digit_reference": [
          1.1919086937529357e-16,
          9.316564212927976e-16,
          1.7088109467536826e-16
        ]
      },
      {
        "z": -1.0,
        "weights": [
          0.05696447062846133,
          0.207276647028654,
          0.16060279414278833
        ],
        "absolute_errors_against_75_digit_reference": [
          9.94300293823069e-17,
          7.457252203673018e-17,
          6.214376836394181e-17
        ]
      },
      {
        "z": -0.9999999999999999,
        "weights": [
          0.05696447062846133,
          0.207276647028654,
          0.16060279414278833
        ],
        "absolute_errors_against_75_digit_reference": [
          1.0689684414558165e-16,
          6.420884481235913e-17,
          6.328625643948642e-17
        ]
      }
    ],
    "scope": "Executed Python binary64 branch implementations, including half arguments for HO. Not Rust or a global floating-point proof."
  },
  "benchmark_geometry": {
    "cases": [
      {
        "case": "similarity-mms-v1",
        "target": "0.015625",
        "edges": [
          {
            "radius": "0.25",
            "X_at_t0_z0": "2",
            "uncut_swirl_amplitude_relative_to_peak": "0.446260320296859657866560941528025042684343258722158657487670637520650333262"
          },
          {
            "radius": "0.375",
            "X_at_t0_z0": "4.5",
            "uncut_swirl_amplitude_relative_to_peak": "0.0549469166662025408811540638197237266357362026604267843087997823177512947943"
          }
        ],
        "endpoints": [
          {
            "k": 1,
            "remaining": "0.0078125",
            "full_nominal_annulus_radius_squared_max": "0.174818148854548371440544195308118176092764202826952027202186505687556372089",
            "nominal_annulus_inside_uncut_ball": false,
            "equatorial_peak_radius_cells_at_N512": "45.2548339959390415616540391747103385142295000120623383416537516157034393108"
          },
          {
            "k": 2,
            "remaining": "0.00390625",
            "full_nominal_annulus_radius_squared_max": "0.0881802336411567687444855143072043442070375220087377727869477943285362677812",
            "nominal_annulus_inside_uncut_ball": false,
            "equatorial_peak_radius_cells_at_N512": "32.0000"
          },
          {
            "k": 3,
            "remaining": "0.001953125",
            "full_nominal_annulus_radius_squared_max": "0.0445486508325529208135645580037508636681091619336716392591583859496745793890",
            "nominal_annulus_inside_uncut_ball": true,
            "equatorial_peak_radius_cells_at_N512": "22.6274169979695207808270195873551692571147500060311691708268758078517196554"
          },
          {
            "k": 4,
            "remaining": "0.0009765625",
            "full_nominal_annulus_radius_squared_max": "0.0225469713710318912250250246117148425197396726434971642926196909617133682229",
            "nominal_annulus_inside_uncut_ball": true,
            "equatorial_peak_radius_cells_at_N512": "16.00000"
          },
          {
            "k": 5,
            "remaining": "0.00048828125",
            "full_nominal_annulus_radius_squared_max": "0.0114356019401518797634013577468481053449288586867023367336066465442778798445",
            "nominal_annulus_inside_uncut_ball": true,
            "equatorial_peak_radius_cells_at_N512": "11.3137084989847603904135097936775846285573750030155845854134379039258598277"
          },
          {
            "k": 6,
            "remaining": "0.000244140625",
            "full_nominal_annulus_radius_squared_max": "0.00581419587181126275972735595506720969254635691775888826503514095773370013932",
            "nominal_annulus_inside_uncut_ball": true,
            "equatorial_peak_radius_cells_at_N512": "8.000000"
          }
        ]
      },
      {
        "case": "similarity-mms-v2",
        "target": "0.0078125",
        "edges": [
          {
            "radius": "0.3",
            "X_at_t0_z0": "5.76",
            "uncut_swirl_amplitude_relative_to_peak": "0.0176334489452492971019566481761073936671154814571939422528312454035080583231"
          },
          {
            "radius": "0.42",
            "X_at_t0_z0": "11.2896",
            "uncut_swirl_amplitude_relative_to_peak": "0.0000979468634785513077398973922105446121239016755267177061668846147860214115678"
          }
        ],
        "endpoints": [
          {
            "k": 1,
            "remaining": "0.00390625",
            "full_nominal_annulus_radius_squared_max": "0.0881802336411567687444855143072043442070375220087377727869477943285362677812",
            "nominal_annulus_inside_uncut_ball": true,
            "equatorial_peak_radius_cells_at_N512": "32.0000"
          },
          {
            "k": 2,
            "remaining": "0.001953125",
            "full_nominal_annulus_radius_squared_max": "0.0445486508325529208135645580037508636681091619336716392591583859496745793890",
            "nominal_annulus_inside_uncut_ball": true,
            "equatorial_peak_radius_cells_at_N512": "22.6274169979695207808270195873551692571147500060311691708268758078517196554"
          },
          {
            "k": 3,
            "remaining": "0.0009765625",
            "full_nominal_annulus_radius_squared_max": "0.0225469713710318912250250246117148425197396726434971642926196909617133682229",
            "nominal_annulus_inside_uncut_ball": true,
            "equatorial_peak_radius_cells_at_N512": "16.00000"
          },
          {
            "k": 4,
            "remaining": "0.00048828125",
            "full_nominal_annulus_radius_squared_max": "0.0114356019401518797634013577468481053449288586867023367336066465442778798445",
            "nominal_annulus_inside_uncut_ball": true,
            "equatorial_peak_radius_cells_at_N512": "11.3137084989847603904135097936775846285573750030155845854134379039258598277"
          },
          {
            "k": 5,
            "remaining": "0.000244140625",
            "full_nominal_annulus_radius_squared_max": "0.00581419587181126275972735595506720969254635691775888826503514095773370013932",
            "nominal_annulus_inside_uncut_ball": true,
            "equatorial_peak_radius_cells_at_N512": "8.000000"
          },
          {
            "k": 6,
            "remaining": "0.0001220703125",
            "full_nominal_annulus_radius_squared_max": "0.00296441468740244843502890308380219129184697857504228824072813157704265575695",
            "nominal_annulus_inside_uncut_ball": true,
            "equatorial_peak_radius_cells_at_N512": "5.65685424949238019520675489683879231427868750150779229270671895196292991386"
          }
        ]
      }
    ],
    "v2_radius_screen": [
      {
        "N": 128,
        "last_endpoint_under_radius_12_cell_screen": 0
      },
      {
        "N": 256,
        "last_endpoint_under_radius_12_cell_screen": 1
      },
      {
        "N": 512,
        "last_endpoint_under_radius_12_cell_screen": 3
      },
      {
        "N": 1024,
        "last_endpoint_under_radius_12_cell_screen": 5
      }
    ],
    "diagnostic_eta_limit": "1/2",
    "scope": "Analytic geometry and amplitude ratios. At t=0 the ramp makes actual velocity zero. Edge amplitude is not removed energy or an error in the localized solution. Cell counts are screening conventions, not convergence results."
  },
  "clock": {
    "binary64_dyadic_counterexample": {
      "floating_sum_equals_target": true,
      "exact_sum_error": "1/18014398509481984"
    },
    "u128_tick_invariant": "passed",
    "scope": "Clock arithmetic fixture; no Rust state transaction was executed."
  },
  "memory": [
    {
      "N": 128,
      "retained_vector_GiB": 0.047607421875,
      "padded_real_vector_GiB": 0.158203125,
      "reserved_base_GiB": 1.25335693359375,
      "excluded": "FFT/provider scratch, planner storage, I/O, diagnostic grids, allocator overhead"
    },
    {
      "N": 256,
      "retained_vector_GiB": 0.3779296875,
      "padded_real_vector_GiB": 1.265625,
      "reserved_base_GiB": 9.982177734375,
      "excluded": "FFT/provider scratch, planner storage, I/O, diagnostic grids, allocator overhead"
    },
    {
      "N": 512,
      "retained_vector_GiB": 3.01171875,
      "padded_real_vector_GiB": 10.125,
      "reserved_base_GiB": 79.6787109375,
      "excluded": "FFT/provider scratch, planner storage, I/O, diagnostic grids, allocator overhead"
    },
    {
      "N": 1024,
      "retained_vector_GiB": 24.046875,
      "padded_real_vector_GiB": 81.0,
      "reserved_base_GiB": 636.71484375,
      "excluded": "FFT/provider scratch, planner storage, I/O, diagnostic grids, allocator overhead"
    }
  ],
  "not_performed": [
    "Rust compilation",
    "PDE integration",
    "source instance admissibility",
    "formal proof build"
  ]
}
````

<!-- END navier-runtime-verification-results.json -->

---

# Input 8: navier-runtime-design.md

<!-- BEGIN navier-runtime-design.md -->

# navier-runtime: standalone Rust solver design

Review/package revision 0.8 · 8 September 2026 · Adopted runtime design revision 0.7

The governing engineering specification is [COMPLETE_DESIGN.md](COMPLETE_DESIGN.md), adopted byte-for-byte at revision 0.7. The [v2 case manifest](similarity-mms-v2.json) is likewise unchanged. [ADVERSARIAL_REVIEW.md](ADVERSARIAL_REVIEW.md) records the revision-0.8 assessment and release decisions; [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md) gives the ordered implementation work.

The public runtime is a standalone three-dimensional incompressible Navier–Stokes library and CLI. Niva integration is an optional downstream adapter outside the public Cargo workspace. No Niva account, code, private data, registry, schema or service is required. The adapter document is excluded from the public review package.

The release targets are the runtime and independent `similarity-mms-v2` convergence experiment. This manufactured case has fixed equations and a prescribed continuum residual force. It does not implement the source pulse cascade or establish a smooth force through the target time. The three-halving reach at `512^3` is a radius screen, not three accepted PDE windows.

Literal-source reproduction is outside the current release scope. The explicit slow-mesh subdivision plan is `FeasibilityExcluded` under [source-feasibility-policy.json](source-feasibility-policy.json); the general literal target remains `FeasibilityUnestablished`. The [source report](SOURCE_FEASIBILITY.md) and [construction ledger](CONSTRUCTION_LEDGER.md) give the source, arithmetic and scope of these distinct statements. Optional compiler work is mathematics verification only. An averaged-stress surrogate is deferred as a separate model decision.

The independence protocol retains from-rest lineage, full-band comparisons, separate force/reference/arithmetic refinement, inherited error and invalidation of unsupported descendant windows. No reference-field reset can advance the independent frontier.

The [script](navier-runtime-verification.py) and [actual results](navier-runtime-verification-results.json) are executed design checks. No Rust compilation, PDE integration, source-instance admission or formal build has been performed. Use the complete [review prompt](navier-runtime-adversarial-review-prompt.md) and [packet](navier-runtime-review-packet.md), or the separate-file ZIP. The [manifest](navier-runtime-review-manifest.json) records component hashes, frozen baselines and completeness checks; these do not certify source-PDF provenance.

<!-- END navier-runtime-design.md -->

---

# Input 9: navier-runtime-construction-design.md

<!-- BEGIN navier-runtime-construction-design.md -->

# navier-runtime: construction and convergence entry point

Revision 0.8 · 8 September 2026 · Adopted engineering baseline revision 0.7

The runtime and manufactured experiment are specified in [COMPLETE_DESIGN.md](COMPLETE_DESIGN.md), which remains unchanged at revision 0.7. The source inventory is [CONSTRUCTION_LEDGER.md](CONSTRUCTION_LEDGER.md); the release priorities and retained future admission requirements are in [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md).

The original literal target is independent integration of one fixed realization of the source force from rest over successively closer pre-singularity intervals. It remains unmet and outside the current release scope. The general status is `FeasibilityUnestablished`. The explicitly defined slow-mesh subdivision strategy is `FeasibilityExcluded` under [source-feasibility-policy.json](source-feasibility-policy.json). Those labels assess different scopes; a resource exclusion for one plan is not a theorem against all full-PDE representations.

The compiler's current role is mathematics verification only. It may check selected identities, parameter inequalities, pointwise fields or finite-stage residuals with stated bounds. It cannot advance the independent PDE frontier. A stress surrogate remains a separate deferred decision and would require its own problem identity.

`similarity-mms-v2` is the active concentrating implementation target. It has fixed continuum fields, a prescribed force, positive viscosity and rest initial data. It has no source pulse cascade or verified all-order force extension. Its mathematical manifest and hash are unchanged. The radius-screen reach is three halvings at `512^3`; no convergence-qualified window currently exists.

Retain full-band comparisons and derivative-sensitive diagnostics, pure prescribed-force evaluation, independent evolving state and reference pathways, same-problem refinements, from-rest lineage, inherited error and invalidation. Empirical sampled evidence must not be presented as an enclosed slab-wide result. Any future reopening of literal reproduction requires actual source admission, complete force/reference accuracy and a new representation-specific feasibility report.

The [assessment](ADVERSARIAL_REVIEW.md) and [source report](SOURCE_FEASIBILITY.md) correct the amplitude inequality, full support width, auxiliary-time interpretation and heuristic carrier estimate. The [script](navier-runtime-verification.py) and [results](navier-runtime-verification-results.json) contain executed algebra and arithmetic only. No source-admissible profiles, full force artifact, Rust solver or PDE trajectory have been produced here.

<!-- END navier-runtime-construction-design.md -->

---

# Input 10: SOURCE_FEASIBILITY.md

<!-- BEGIN SOURCE_FEASIBILITY.md -->

# navier-runtime: localized construction feasibility extraction

Revision 0.8 · 8 September 2026

This report retains the revision-0.7 extraction and assesses the supplied revision-0.7 review. The runtime specification remains revision 0.7, adopted without changes. Release priorities and scoped feasibility states are recorded in Section 6 and the machine-readable policy. It records source formulas, our derived screening calculations, and the next numerical obligations. It is neither a proof audit nor an admitted source instance. The governing runtime specification is [COMPLETE_DESIGN.md](COMPLETE_DESIGN.md); object status is in [CONSTRUCTION_LEDGER.md](CONSTRUCTION_LEDGER.md). Actual calculations are in the accompanying [script](navier-runtime-verification.py) and [results](navier-runtime-verification-results.json).

## 1. Localization: accepted, with finite-window qualifications

The proof of Proposition 10.1 permits any fixed positive radial cutoff radius. Its restriction on support concerns time and axial extent. Equations (10.3)–(10.5), followed by Lemmas 10.2–10.3, provide the localized field and force. The relevant rendered pages, including the endpoint and extension arguments, were inspected. [NS, pp. 117–120]

Using distinct names for the physical cutoff and auxiliary rectangle radius, the source choices are

```
C0 (tau0 + z_cut^(1/D)) < q_star/2
chi_x = 1 on {r <= r_cut/2, |z| <= z_cut/2}
support(chi_x) contained in {r < r_cut, |z| < z_cut}
chi_t = 0 for tau >= tau0; chi_t = 1 for tau <= tau0/2
u = curl(c A) + c B e_theta; p = c p_loc; c = chi_x chi_t
f = partial_t u + (u dot grad)u - Laplacian(u) + grad(p).
```

**Consequence for this design:** do not require an Eulerian box containing the full similarity annulus at forcing onset. Add `r_cut`, `z_cut`, both cutoff functions, activation interval, and the complete rescaling to the immutable mathematical problem. The initial source support test uses the maximum of `q` over that support, not just the equatorial value `q=tau`.

The smaller box does not eliminate the global profile construction. Its parameter restrictions, moment equations, pressure normalization and coefficient bounds still define the field being localized. Source evaluation must retain those dependencies or justify an equivalent reformulation. Cutting off the resulting field is different from discarding remote terms inside a profile inverse or radial integral.

The cutoff must also remain fixed across endpoint extensions. For an equatorial outer annulus radius `sqrt(2 X_b tau)` and an uncut physical radius `R_flat`, the entire outer annulus enters the uncut region once

```
tau <= R_flat^2/(2 X_b).
```

Thus an onset localization can remove the large outer domain from early windows; it cannot promise to remove it from every successive interval approaching the singularity. Moving the cutoff inward between extensions changes the force and the problem identity.

There is a useful scaling choice, but translation alone does not lengthen an interval. If source time starts at `t_on=1-tau0`, set

```
u_new(x,s) = a u(a x, t_on+a^2 s)
p_new(x,s) = a^2 p(a x, t_on+a^2 s)
f_new(x,s) = a^3 f(a x, t_on+a^2 s)
new remaining interval = tau0/a^2; new radial support = r_cut/a.
```

For `tau0=q0` and `r_cut=rho sqrt(q0)`, choosing `a=sqrt(q0)` gives duration one in a box of width greater than `2 rho`. Fitting the same support inside a unit cube requires `a>2 rho sqrt(q0)`, hence duration below `1/(4 rho^2)`, with a smaller value for a support margin. These are equivalent valid parameterizations. Record the prefactor and viscosity convention; do not independently prescribe unit box, unit viscosity, and duration one for that same scaled realization.

## 2. Question 1: does the quarter-parameter band lie below q_star?

The exact numerical `q_star` remains unknown. The common-domain selection is in Lemma 9.7 and Proposition 9.9; the pulse conditions include Lemma 7.1. One explicitly available sufficient test is

```
S_star^2 (epsilon + epsilon^2 + 1/k) <= 1
epsilon = 2^(-h ell), k = ceil(epsilon^(-1/2)), S_star = ell^2.
```

[NS, pp. 76–77, 111–115]

We evaluated this test with the deliberately relaxed boundary `h_bar=exp(-11)`. This value is not an admitted source exponent. At `ell=119749`, the executed calculation gives

| Quantity | Relaxed calculation |
|---|---:|
| `epsilon` | `0.2499979261241724...` |
| `k` | `3` |
| `S_star` | `14,339,823,001` |
| Displayed sufficient-test left side | `1.3280240687888123e20` |

The smallness test fails by twenty orders of magnitude. The quarter condition by itself therefore does not authorize this band under that explicit proof route. This is a failed sufficient test, not proof that the desired frame estimates fail there.

For a reproducible conservative screen, replace `1/k` by its upper bound `sqrt(epsilon)` and find the first integer in the decreasing regime satisfying

```
ell^4 (epsilon + epsilon^2 + sqrt(epsilon)) <= 1.
```

Each term decreases for `ell >= 8/(h_bar log(2))`; bounded bracketing and bisection locate the transition and check its predecessor. The actual script output is

| Quantity | First passing conservative screen |
|---|---:|
| `ell` | `11,217,670` |
| `epsilon` | `3.9881882378283902e-57` |
| `k` | `15,834,785,175,573,390,502,848,019,699` |
| `S_star` | `125,836,120,228,900` |
| Screen at the preceding integer | `1.0000018939199667...` |
| Screen at this integer | `0.9999964621363287...` |

This is **not** a computed `q_star`, the first admissible band, or a necessary lower bound for every realization. The actual exponent, profile-dependent frame and covariance margins, slow-box variation, and other primary inverse bounds are not yet evaluated. Actual margins may require a later band; sharper direct estimates could avoid this particular sufficient screen.

**Optional compiler next calculation:** obtain numerical leading profiles on the retained region together with the global bounds their construction requires. Evaluate `|n_Phi|`, the frame determinant, covariance positivity and the fixed mean inverse with rounding/derivative control. Return a verified or explicitly empirical interval for allowable band indices. Record all additional conditions; do not identify `q_star` with the single test above.

## 3. Question 2: what does S_star actually cost?

The source uses a product partition in `(R,Z,T)` of mesh `S_star^-3`. Lemma 6.3 distinguishes bounded point overlap from chart and radial-integral label counts. The pulse interval and envelope are governed by (6.11)–(6.12) and (7.16). [NS, pp. 64–68, 78]

The resulting representation scales are

```
chart mesh                 delta = S_star^-3 = ell^-6
pointwise overlap          bounded independently of ell
chart-volume label bound   C_B S_star^9
radial-integral label bound C_I S_star^3
pulse-coordinate interval  L_s = 2 r_aux/c_i, comparable to S_star
Gaussian envelope width    proportional to sqrt(L_s), with profile constants
auxiliary traversal time   Q^(1+h) L_s
auxiliary envelope proxy   Q^(1+h) sqrt(L_s), with profile constants
slow cutoff full extent    at most 2 Q ell^-6 in physical time.
```

On the ordinary chart-time scale `tau/Q`, the auxiliary factors are `epsilon L_s` and `epsilon sqrt(L_s)`. By (6.12), `t_star v=1` and `t_star=Q^(1+h) partial_t` after physical evaluation. Thus `dv/dt=Q^(-1-h)` on a local lift. This is a rectangle-traversal duration, not the lifetime of the full labelled pulse. Actual support intersects the slow cutoff (full extent at most `2Q ell^-6`), the auxiliary rectangle/envelope, and every spatial, dyadic and correction cutoff. The scales can differ without contradiction. A method-dependent temporal resolution rule cannot be inferred from support alone. [NS, pp. 64–66]

Our calculated scale factors are

| Band | Chart mesh `S^-3` | `S^3` | `S^9` |
|---:|---:|---:|---:|
| 119,749 | `3.3913e-31` | `2.9487e30` | `2.5639e91` |
| 11,217,670 | `5.0186e-43` | `1.9926e42` | `7.9113e126` |

These are powers appearing in the specified mesh and count estimates. They are not measured counts of nonzero pulses in the chosen localized box, and an asymptotic upper bound is not a computational lower bound. A retained active chart region with positive volume can nevertheless contain an enormous number of mesh cells. An eager implementation that stores every such label is not a credible first backend.

Point evaluation may use direct mesh indexing with bounded overlap. That observation does not settle integration of radial means, auxiliary averages, or resolution of the summed physical field. The compiler must distinguish labels locally queried, labels reached by inverse operations, materialized labels, and labels whose contribution is bounded and omitted. The global number of labels alone does not price an implicit evaluator; pointwise overlap alone does not price a PDE trajectory.

At the passing screen, the calculated factors `epsilon S` and `epsilon sqrt(S)` are approximately `5.0186e-43` and `4.4738e-50`. They require the still-unknown `r_aux`, frame and envelope constants before becoming auxiliary traversal/envelope durations, and intersection with all cutoffs before describing an actual pulse lifetime. The label geometry is not harmless merely because it is polynomial in `log(1/Q)`.

**Optional compiler next calculation:** fix cutoff support and an immutable label/coloring rule. Implement a lazy support query and one representative radial-mean evaluation. Measure or bound the actual traversed label count, physical variation, interpolation/omission error, and auxiliary inverse work. Retain a full-PDE result label only if the physical field is resolved or its discarded contribution is quantitatively controlled.


### 3.1 Mesh, amplitude and representation are distinct obligations

The mesh statement is source checked: `delta=S_star^-3=ell^-6`. The support extends by at most one mesh spacing on each side, so its full extent is at most `2delta`. The review's quoted scale is the spacing or one-sided extent; treating it as the full support width misses a factor two. That factor does not improve the explicit-mesh resource outlook. [NS, p. 64]

The amplitude inference in the review has its inequality reversed. At `q=Q`, the pure factor in Section 3.3 is

```
q^(h/2) = sqrt(epsilon)
epsilon <= 1/4  implies  sqrt(epsilon) <= 1/2.
```

The source comparison suppresses logarithmic factors, profile values and weights that vanish at support boundaries. Consequently neither a literal half-background lower bound nor an absolute upper bound on every pulse follows from that factor alone. The passed relaxed screen gives `sqrt(epsilon)=6.3152104e-29`; this number is not a bound authorizing removal of the pulses. Derivative, residual, covariance and relative-error requirements still matter. [NS, Section 3.3, p. 11; Proposition 7.5, p. 82]

The following are executed arithmetic, not admitted source parameters. `Lambda=128` is an explicitly illustrative choice giving a chart core radius `sqrt(8/Lambda)=1/4`; no source-admissibility claim is made for it.

| Quantity | Relaxed quarter screen | Relaxed passing sufficient screen |
|---|---:|---:|
| Band `ell` | 119,749 | 11,217,670 |
| Pure `sqrt(epsilon)` factor | `0.4999979261` | `6.3152104e-29` |
| Mesh spacing `delta` | `3.3913189e-31` | `5.0186169e-43` |
| Full slow-support extent upper bound `2delta` | `6.7826378e-31` | `1.0037234e-42` |
| `k/ell^6` | `1.0173957e-30` | `7.9468720e-15` |
| `delta/(1/4)` | `1.3565276e-30` | `2.0074467e-42` |
| Explicit intervals across chart length `1/4` at spacing at most `delta` | `737176328365264901456001617251` | `498145220411950791845051572737215892250000` |

The final row is an exact ceiling calculation for the declared subdivision policy. It is a lower bound for that policy, not the number of actually active physical pulses or a lower bound for every numerical representation. It assumes an explicit grid that resolves each slow-mesh interval across the declared segment; it does not assert that an admitted localized realization has an active pulse at every point of that segment.

The source assigns disjoint auxiliary supports to labels with intersecting slow supports. Their cross-products vanish after physical evaluation. Generic partition cancellation therefore does not establish that the actual pulses disappear. Conversely, the slow partition spacing alone is not a norm-specific lower bound on the complexity of the summed physical field. Such a bound would need actual nonvanishing support, amplitudes and phase gradients, the requested norm and tolerance, and a specified class of representations. None has been supplied as a universal exclusion theorem. A compressed proposal would still have to independently evolve the same PDE and control pressure, nonlinear products, forcing and derivatives. No working alternative has been established. [NS, Section 3.3; Lemma 6.1]

### 3.2 Finite-band curl-remainder inference

Lemma 7.7 was checked against the displayed potential and normalized curl, including the rendered equations on pp. 85–86. For a harmonic coefficient `t_m`,

```
C_m = i (n_Phi cross t_m)/(k m |n_Phi|^2)
curl_star(C_m exp(i k m Phi)) = (t_m+r_m) exp(i k m Phi)
C_m in W_(alpha+1/2),  r_m in W_(alpha+1/2-kappa_s).
```

The proof combines `1/(km)` with derivative losses and polynomial coefficient bounds. Turning this class statement into a finite-band inequality requires the actual constants, phase-normal factors, cutoff derivatives and domain. `k>ell^6` is a useful scale-balance diagnostic; it has not been proved here as the necessary admission condition.

At the relaxed `h=exp(-11)`, the pure carrier/mesh balance crosses between `ell=17274000` and `ell=17274001`. At the latter, `k=26568006778617507675334759249850259116073994`, approximately `2.6568e43`, and `k/ell^6=1.0000030647`. The preceding ratio is `0.9999976237`. These are executed checks of the heuristic, correcting the review's rough `1e50` carrier estimate. They do not compute `q_star`, establish a finite remainder bound, or admit either band.

## 4. Question 3: actual phase and phase-map derivatives

The auxiliary map constants in (6.2)–(6.6) are explicit; the pulse normal in (7.2)–(7.4) depends on the background. [NS, pp. 63–65, 73–76]

```
J_g = [[3,1],[1,5]]
Lambda_g = 4-sqrt(2); T_g = 4+sqrt(2)
b_g = sqrt(2)-1
v_r = (1,-b_g); v_t = (b_g,1)
rho_g = log(Lambda_g)/log(T_g)
d_r = 2 ((1+h)rho_g - h*1e-5)
Y = v_r r^d_r + v_t t modulo Z^2
i = floor(log_T_g(Q^(-1-h)/S_star))
c_i = T_g^i Q^(1+h)
M_i = Lambda_g^i Q^(d_r/2)
D_r = partial_R + M_i d_r R^(d_r-1) v_r dot partial_Yi
D_z = epsilon partial_Z
t_star = -epsilon partial_T + c_i v_t dot partial_Yi.
```

The computed relaxed constants are `rho_g=0.5624713724158318...` and `d_r=1.1249615329547599...`. At the quarter band, `i=49129`, `c_i=1.64736e-11`, and `M_i=8.60514e-7`; at the passing screen, `i=4603591`, `c_i=2.15767e-15`, and `M_i=5.64093e-9`. These checks require no instantiated profile. Exponent-separated evaluation avoids constructing `J_g^i` merely to determine these scalar scales; accurate phase reduction modulo the torus still requires its own error budget.

For the actual phase, write `F=V/R`, `G` for the chart axial velocity, and `g0=(R F_R,G_R)` at the frozen representative. The source gives

```
N = g0/|g0|; K = (-N_z,N_theta)
lambda0^2 = -2 F0 N_theta (2 F0 N_theta + |g0|)
B_s^2 = lambda0/[epsilon k^2 (1+u_star^2)^(3/2)]
Phi = p theta + p_z Z/epsilon + x0 R - v(p F+p_z G)
n_Phi = (x0-v(p F_R+p_z G_R), p/R,
         p_z-epsilon v(p F_Z+p_z G_Z)).
```

`k p` is selected as a nearest nonzero integer by a fixed rule. The derivation of `p,p_z,x0,u_star` requires the source's cone and frame data; they are not independently adjustable unit coefficients. The actual harmonic is `exp(i k m Phi)`, so our conversion to physical frequencies is

```
physical wavevector = (k m/sqrt(Q)) n_Phi
angular Fourier integer = k m p
physical angular wavelength at r = 2 pi r/|k m p|
normal-direction wavelength = 2 pi sqrt(Q)/(|k m| |n_Phi|).
```

Include amplitude derivatives from the slow mesh and auxiliary pullback as well as these carrier frequencies. The auxiliary radius `r_aux` is the Section 6 rectangle radius; it is not the independently chosen physical localization radius `r_cut`.

**Optional compiler next calculation:** evaluate full phase gradients and their extrema on each materially active retained patch, including pressure/mean dependence on other patches. Until `F0`, `g0`, `u_star`, covariance margins and rounding choices exist, neither the estimate “a few tens of wavelengths” nor a large carrier alone is a complete physical resolution count.

## 5. Question 4: background and correction orders on a window

Lemma 5.4 supplies cutoff selection inequalities; Proposition 5.5 uses them for background orders, and (9.21) uses them for correction stages. Lemma 9.8 assigns correction decay gain `g_j=h j/10`; the background gain is `2hn`. The initialization block has a separate possible cutoff. [NS, pp. 57–61, 112–115]

The existing proof algorithm contains the useful finite conditions

```
a_(j+1) >= 2 a_j
C_hat_(j,m) (1+|log q|)^P_hat_(j,m) q^(g_j/2) <= 2^-j
    for 0 < q <= 1/a_j and 0 <= m <= j
chi(s)=1 for s<=1/2; chi(s)=0 for s>=1.
```

No numerical cutoff sequence was supplied by the source or computed here. The right numerical question is therefore not a universal count `n,j` at band 119749. It is the count for a chosen immutable sequence satisfying these inequalities.

For a covered domain with a proved positive `q_min`, implement

```
candidate background orders = {n>=1: c_n q_min < 1}
candidate correction stages = {j>=1: a_j q_min < 1}
```

Then intersect with physical supports. Terms in the transition `1/2<a_j q<1` require all cutoff derivatives. Prefix enumeration stops once the next scale times `q_min` is at least one, provided monotonic growth is verified. For the doubling sequence, if `a_1 q_min<1`, the candidate count is at most `ceil(log2(1/(a_1 q_min)))`; otherwise it is zero. Coefficient and residual bounds, not an invented default count, determine the selected scales.

The freedom to choose later cutoffs can delay higher stages on an initial window while leaving the eventual construction intact. That is a valid realization choice if fixed in advance and all bounds hold. It does not establish that a window contains the pulse mechanism. Record whether the initialization waves and which correction stages are actually active. A source-derived early tracking result and a resolved pulse-mechanism result have different evidence requirements.

**Optional compiler next calculation:** choose a deterministic cutoff-scale search using actual coefficient bounds, produce the first needed scales, and compute their interval support intersections. A missing bound returns `MissingBound`. It must not be replaced by “use only a few stages.”

## 6. Gate disposition and current release

Adopt the revision-0.7 runtime and `similarity-mms-v2` as the release implementation targets. Neither depends on the source compiler. The current release excludes literal-source reproduction as an engineering scope decision. The optional compiler is mathematics verification only; source extraction is no longer a prerequisite for the runtime or benchmark.

| Object being assessed | Recorded status | Meaning |
|---|---|---|
| Explicit slow-mesh subdivision under `explicit-slow-mesh-v1` | `FeasibilityExcluded` | Resolving chart length `1/4` with spacing at most `ell^-6` exceeds the declared 1,024-interval-per-axis cap at both displayed bands and every larger integer |
| Literal target across unspecified full-PDE representations | `FeasibilityUnestablished` | No admitted realization, working representation or valid universal exclusion theorem has been established |
| Literal reproduction in the current release | Excluded from scope | No implementation time is committed to a direct-source PDE backend in this release |
| Construction compiler | `MathematicsVerificationOnly` | Report the identities, inequalities, pointwise or finite-stage residual checks actually executed |
| Standalone runtime and `similarity-mms-v2` | Adopted implementation targets | Their numerical evidence must come from independent PDE integration and separate convergence channels |
| Averaged-stress surrogate | `SeparateDecisionDeferred` | Distinct model; neither implemented nor required for this release |

The cap is a declared screen policy, not a hardware measurement, a global grid limit for the runtime, or a universal bound. The policy and computed counts are in [source-feasibility-policy.json](source-feasibility-policy.json). Even raising that cap by many orders would leave these explicit subdivision counts out of reach for that strategy. Changing the representation, support assumptions, norm or approximation requirements requires a new policy assessment, not silent reuse of the excluded gate.

The four extracted source questions remain optional mathematical work and obligations for any future reopening of literal reproduction. There is no requirement to answer them before starting the runtime. An early localized window without active pulses is not a demonstration of the pulse mechanism. The exclusion does not claim to rule out such windows.

Proposition 7.5's covariance is formed by averaging angular and independent auxiliary coordinates before physical pullback. It is not automatically the coarse-grained covariance of the evaluated physical velocity. A future surrogate must specify its complete physical stress tensor, sign and geometry of its divergence, localization, force identity, and approximation objective. It can be a full numerical solution of its own modified forced-NS problem; it cannot be called reproduction of the source pulse field. Neither necessity nor uniqueness of that route has been demonstrated. [NS, p. 11; Proposition 7.5, p. 82]

At `512^3`, the benchmark has three halvings under its twelve-cells-per-peak-radius screen. This is geometry, not three converged windows. Coarser comparison grids and force/time/arithmetic errors may shorten the accepted frontier. No PDE windows have been accepted.

## Source and inspection scope

**NS:** OpenAI, [Finite Time Blowup for Navier–Stokes](https://cdn.openai.com/pdf/32d9f210-8b73-45e0-91bc-82a30aef8a9a/navier-stokes.pdf), especially Lemma 5.4, Proposition 5.5, Section 6.1–6.3, Lemma 7.1, Proposition 7.2, Lemmas 9.7–9.8, Proposition 9.9, Proposition 10.1 and Lemmas 10.2–10.3. Page numbers above are printed page numbers. Targeted text and rendered equations were inspected; Sections 5–9 and the appendices were not audited line by line in their entirety. The source PDF checksum and exact repository commits remain unverified. Algebra and numerical screens in this report are project calculations with explicitly stated relaxed assumptions.

The revision-0.7 reviewer reports reading `NavierStokes/SlotColoring.lean` at commit `8937a8f4cbc7abaab5e9e97d1cc7f5d2319d9538`. Its reported mesh description agrees with the independently inspected PDF. Pinned-file and main-file retrieval attempts failed in this task; neither that file nor the Lean comparator is promoted to independently verified provenance, and no formal build was run. No quoted source module text is redistributed in this package.

<!-- END SOURCE_FEASIBILITY.md -->

---

# Input 11: similarity-mms-v2.json

<!-- BEGIN similarity-mms-v2.json -->

````json
{
  "schema_version": 1,
  "design_revision": "0.7",
  "case_id": "similarity-mms-v2",
  "classification": "manufactured_concentrating_prescribed_force_problem",
  "implementation_status": "Specification artifact; no implemented CLI parser or PDE run",
  "mathematical_problem": {
    "equation": "3D incompressible Navier-Stokes with prescribed force",
    "coordinates": "Cartesian nondimensional x,y,z,t",
    "periodic_box": {
      "lengths": [
        "1",
        "1",
        "1"
      ],
      "center": [
        "0",
        "0",
        "0"
      ]
    },
    "parameters": {
      "nu": "1",
      "h": "1/8",
      "b": "1/4",
      "j0": "1/32",
      "T_star": "1/128",
      "t_ramp": "1/512",
      "R_in": "3/10",
      "R_out": "21/50"
    },
    "derived_parameters": {
      "A": "1/2+h",
      "D": "1/2-h"
    },
    "time_domain": "0 <= t < T_star",
    "similarity_coordinates": {
      "tau": "T_star-t",
      "q": "unique positive admissible root of q-z^2*q^(2*h)=tau",
      "eta": "z*q^(-D)",
      "X": "(x^2+y^2)/(2*q)"
    },
    "profiles": {
      "a": "eta+j0",
      "M": "a*X*exp(-X)",
      "U": "a*(1-X)*exp(-X)",
      "F": "b*exp(-X)",
      "Pi": "-(b^2/2)*exp(-2*X)"
    },
    "smooth_step": {
      "rho": "rho(s)=exp(-1/s) for s>0; rho(s)=0 otherwise",
      "S": "S(s)=0 for s<=0; rho(s)/(rho(s)+rho(1-s)) for 0<s<1; 1 for s>=1"
    },
    "cutoffs": {
      "c_x": "1-S((x^2+y^2+z^2-R_in^2)/(R_out^2-R_in^2))",
      "c_t": "S(t/t_ramp)",
      "c": "c_x*c_t"
    },
    "reference_fields": {
      "G": "(1/2)*q^(-A)*(eta+j0)*exp(-X)",
      "B": "b*q^(-A-1/2)*exp(-X)",
      "A_vec": "G*(-y,x,0)",
      "B_vec": "B*(-y,x,0)",
      "u_S": "curl(c*A_vec)+c*B_vec",
      "p_S_raw": "c^2*q^(-2*A)*Pi",
      "p_S": "p_S_raw minus its periodic spatial mean at the same time"
    },
    "force": "partial_t(u_S)+(u_S dot grad)u_S-nu*Laplacian(u_S)+grad(p_S_raw)",
    "initial_condition": {
      "time": "0",
      "velocity": [
        "0",
        "0",
        "0"
      ]
    },
    "force_dependence": "Prescribed by the fields above; never depends on integrated velocity or numerical grid",
    "terminal_extension": "Unspecified: no smooth force extension through T_star is claimed"
  },
  "problem_identity": {
    "algorithm": "SHA-256",
    "canonicalization": "mathematical_problem only; json.dumps(sort_keys=True,separators=(',',':'),ensure_ascii=True), ASCII bytes, no final newline",
    "sha256": "ba81b7709e68cb118d3fc60c1d9bbcd27f424358b121ed4be08660ab8b88210f",
    "scope": "Syntactic identity of this exact definition, not mathematical equivalence detection or a source-admissibility certificate",
    "exact_input_policy": "All mathematical scalar input values are integer/rational strings, never binary64 literals"
  },
  "diagnostics": {
    "identity_scope": "Separate from mathematical_problem; changes require a new comparison-protocol identity",
    "nominal_eta": [
      "-1/2",
      "1/2"
    ],
    "core": "0 <= X <= 1/2",
    "annulus": "1/2 < X <= 8",
    "interior_mask": "Intersect each nominal region with mathematical c_x=1",
    "coverage_measure": "Physical volume at each actual comparison time",
    "empty_region_status": "RegionEmpty",
    "additional_regions": [
      "startup 0 <= t <= t_ramp",
      "cutoff collar 0 < c_x < 1"
    ],
    "coverage_quadrature": {
      "q": "tau/(1-eta^2)",
      "z": "eta*q^D",
      "X_limit": "(R_in^2-z^2)/(2*q)",
      "weight": "q^(1+D)*(1-2*h*eta^2)/(1-eta^2)",
      "fraction": "integral(weight*clamp(X_limit-X_low,0,X_high-X_low),eta=-1/2..1/2)/((X_high-X_low)*integral(weight,eta=-1/2..1/2))",
      "accuracy_requirement": "Independent refinement or enclosure of this quadrature",
      "executed": false
    }
  },
  "experiment_proposal": {
    "endpoint_rule": "tau_k=T_star*2^(-k), t_k=T_star-tau_k, integer k>=1",
    "start": "Each refinement lineage originates from the exact initial rest state",
    "successive_windows": "Extend the same immutable input; compare against independently evolved from-rest refinement trajectories",
    "grid_screen": {
      "rule": "N*sqrt(T_star*2^(-k)) >= 12",
      "meaning": "Cells across the equatorial swirl-peak radius, not diameter",
      "classification": "Heuristic resource screen; not an acceptance or convergence criterion",
      "maximum_individual_grid_k": {
        "128": 0,
        "256": 1,
        "512": 3,
        "1024": 5
      },
      "caveat": "Coarser comparison grids can shorten the qualified frontier; memory preflight remains mandatory"
    },
    "acceptance_contract": "COMPLETE_DESIGN.md Sections 6 and 7; no accepted window exists"
  },
  "historical_case": {
    "id": "similarity-mms-v1",
    "T_star": "1/64",
    "R_in": "1/4",
    "R_out": "3/8",
    "compatibility": "Different mathematical problem; cannot resume a v1 checkpoint as v2"
  },
  "source_instance": false,
  "verified_pde_convergence": false
}
````

<!-- END similarity-mms-v2.json -->

---

# Input 12: source-feasibility-policy.json

<!-- BEGIN source-feasibility-policy.json -->

````json
{
  "schema_version": 1,
  "revision": "0.8",
  "runtime_design_revision": "0.7",
  "adopted_baseline_sha256": {
    "COMPLETE_DESIGN.md": "fabca082cf73fee5f64c7f67800308b5115936bbec04838100a0d2ee425b81d9",
    "similarity-mms-v2.json": "e1236f7b3c51537acd17381402ca420ba7872a7b9dbc64b2f0d9d5108a468f7e"
  },
  "literal_target_status": "FeasibilityUnestablished",
  "explicit_mesh_policy": {
    "policy_id": "explicit-slow-mesh-v1",
    "chart_segment_length": "1/4",
    "required_spacing_at_most": "ell^-6",
    "maximum_stored_intervals_per_axis": 1024,
    "cap_origin": "Declared policy for this screen, not measured hardware capacity or a universal resource bound",
    "minimum_tested_band": 119749,
    "band_scope": "Both displayed relaxed bands and every larger integer under this same policy; required intervals increase as ell^6",
    "scope": "Explicit subdivision of every mesh interval across the declared chart segment, conditional on undertaking that mesh-resolving plan; actual localized active support has not been admitted",
    "status": "FeasibilityExcluded",
    "reason": "ceil((1/4)*ell^6) exceeds the declared 1024-interval cap in each tested row",
    "universal_representation_exclusion_claimed": false
  },
  "computed_rows": [
    {
      "ell": 119749,
      "sqrt_epsilon_pure_asymptotic_factor": "0.499997926119871457568808853287950401416600628944563018721131691979822674458",
      "chart_mesh_spacing": "3.39131887962803689435180465159829424524821724868525252196518455047739120995E-31",
      "slow_support_full_extent_upper_bound": "6.78263775925607378870360930319658849049643449737050504393036910095478241990E-31",
      "carrier_over_ell6": "1.01739566388841106830554139547948827357446517460557575658955536514321736298E-30",
      "mesh_over_illustrative_core_radius": "1.35652755185121475774072186063931769809928689947410100878607382019095648398E-30",
      "auxiliary_traversal_chart_proxy": "3584926011.23770668495331218565794420003682870561647028090530366998489077064",
      "auxiliary_gaussian_chart_proxy": "29937.0016554435250812391935269433915943918421499676012401381528863279924729",
      "auxiliary_traversal_proxy_over_slow_full_extent_bound": "5285445188850693990753466814108738977102.29096427342422016664864332209468536",
      "auxiliary_gaussian_proxy_over_slow_full_extent_bound": "44137697925249429980655093688538016.8277170662324814755878266093522459033925",
      "policy_required_intervals_per_axis": "737176328365264901456001617251",
      "policy_exceeds_cap": true
    },
    {
      "ell": 11217670,
      "sqrt_epsilon_pure_asymptotic_factor": "6.31521039857611572137854393727287575481404936883605345946090917287410564370E-29",
      "chart_mesh_spacing": "5.01861685621027701945479483312211290192633848273357532393670529210182518260E-43",
      "slow_support_full_extent_upper_bound": "1.00372337124205540389095896662442258038526769654671506478734105842036503652E-42",
      "carrier_over_ell6": "7.94687197966012284730899303197326463185149803510488014171908767652647492989E-15",
      "mesh_over_illustrative_core_radius": "2.00744674248411080778191793324884516077053539309343012957468211684073007304E-42",
      "auxiliary_traversal_chart_proxy": "5.01858134590858141192438235339119827284067628619905418377853804063270674042E-43",
      "auxiliary_gaussian_chart_proxy": "4.47381795498403983351657015529178365279124478273924458802811817483729396606E-50",
      "auxiliary_traversal_proxy_over_slow_full_extent_bound": "0.499996462142586989343365371278804553508732791971570236171613693438661501281",
      "auxiliary_gaussian_proxy_over_slow_full_extent_bound": "4.45722206253693493696431942889035382132593303218556292145885637069606701999E-8",
      "policy_required_intervals_per_axis": "498145220411950791845051572737215892250000",
      "policy_exceeds_cap": true
    }
  ],
  "direct_reproduction_in_current_release": false,
  "compiler_scope": "MathematicsVerificationOnly",
  "surrogate_status": "SeparateDecisionDeferred",
  "source_profile_admitted": false,
  "accepted_pde_windows": 0,
  "runtime_implementation_exists": false,
  "evidence_source": "navier-runtime-verification-results.json/source_mesh_review",
  "provenance": {
    "pdf_mesh_independently_inspected": true,
    "source_pdf_sha256": null,
    "lean_file": "NavierStokes/SlotColoring.lean",
    "lean_commit_reported_by_reviewer": "8937a8f4cbc7abaab5e9e97d1cc7f5d2319d9538",
    "lean_file_independently_verified": false,
    "formal_build_performed": false
  }
}
````

<!-- END source-feasibility-policy.json -->

END_NAVIER_RUNTIME_REVIEW_PACKET_REV_08
