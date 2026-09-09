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
