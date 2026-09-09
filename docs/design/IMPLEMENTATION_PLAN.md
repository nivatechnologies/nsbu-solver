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
