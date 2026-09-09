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
