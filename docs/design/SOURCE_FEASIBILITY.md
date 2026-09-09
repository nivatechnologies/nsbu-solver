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
