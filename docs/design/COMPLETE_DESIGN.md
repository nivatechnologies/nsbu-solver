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
