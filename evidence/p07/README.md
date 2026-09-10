# P07 — independent HO method (complete)

The implementation adds the reviewed five-stage Hochbruck–Ostermann
method with independent coefficient construction and a separate stage kernel.
A selected method owns its coefficient tables and kernel scratch. Both methods
use the same exact-clock, local-comparison and transactional-commit machinery.
CM retains twelve RHS calls per full/two-half attempt; HO uses fifteen. The
source provider remains independent of the evolving velocity and reference state.

## Demonstrated numerical behavior

- Hypergeometric coefficient fixtures cover zero, tiny arguments, both sides of
  the full/half Taylor junctions, the first-weight cancellation root and extreme
  finite negative arguments. The maximum measured conditioning-scaled binary64
  error is approximately 1.23e-15. Absolute errors are reported at cancellation
  roots; a tiny expected coefficient is not used as an artificial relative scale.
- Nonzero-operator row/weight identities and all eight zero-operator fourth-order
  conditions pass. The analytic truncation report follows the actual arithmetic
  branch and does not enclose floating-point roundoff.
- Requested source times are start, half, half, end, half. A deliberately
  increasing-time-only provider is refused by the explicit negative control.
- Independent 80/120-digit direct-DFT full-step and two-half-step fixtures agree
  with actual Rust full spectra. No analytical state is assigned during evolution.
- Fixed scalar problems demonstrate stiff nonautonomous convergence. The
  lambda=100000, frequency=13 coarse family retains approximately second-order
  behavior as a documented order-reduction control. The frequency=1300 family is
  a separately identified problem, not a refinement of frequency=13; it keeps
  truncation errors measurable while testing large decay. Finer frequency=13
  reference values are separately retained to distinguish truncation from the
  binary64 floor. No order observed in these fixtures is presumed for every PDE
  window or used to authorize a Richardson divisor.
- Both methods pass nonlinear smooth PDE temporal refinement from rest. The
  existing smooth spatial family remains a band-limited consistency study, not
  a general spatial convergence-order claim.
- Independent N=4 concentrating paths reach 1/256 with a full-band CM/HO L2
  difference of about 7.53e-11 at committed step 1/16384. They remain unresolved
  spatial/force diagnostics, with zero accepted PDE windows.
- Dedicated allocator instrumentation exercises both methods through repeated
  local rejection, stale-token refusal and swap commits without allocation,
  deallocation or reallocation during attempts.

## Responsibility review

Coefficient formulas and kernels remain method-specific. Shared code handles
finite-value validation, modal arguments, attempt admission, error comparison
and swap commits. It does not share independent Python oracles with production
arithmetic. RHS accounting consumes the selected finite call bound; prescribed
force evaluation has no method, reference-state or assignment interface.
Existing CM public constructors remain available and select CM explicitly.
HO construction requires its larger scratch reservation before allocation.

The standalone full-step test releases its kernel/table scratch before creating
the transactional workspace. The retained full-spectrum comparison array fits
within the separate test metadata allowance. The two concentrating branches have
independent state/candidate/workspace lifetimes and separate preflight totals.
No allocation saving depends on shared mutable trajectory payloads.

## Quality scope and pending gates

Every maintained Rust implementation and test file remains in complexity,
source-size, duplication, dead-code review and complete executable line/branch
coverage scope. Mutation generation includes all production Rust sources. The
two long trajectory studies are skipped only in per-mutant reruns; mandatory
full-suite and coverage runs include them. Focused kernel, force, clock, budget,
transaction and smooth temporal-order tests remain in mutation reruns. Every
mutant must be caught or explicitly classified; timeouts are not kills.

The first targeted run found two survivors. The coefficient truncation report
was coupled to the branch actually used, and counter-range tests gained a valid
large transform declaration plus the fifteen-call overflow boundary. The targeted
rerun passed 204 mutants: 197 caught, seven unviable, no survivors or timeouts.
The final source-matched complete run passed 1537 mutants: 1422 caught, 115
unviable, no survivors or timeouts. All 120 tests/probes pass, with 6,062 executable
lines and 514 instrumented branches covered. The maxima are cyclomatic 19,
cognitive 14, Halstead difficulty 75.896, physical source lines 264 and CRAP 19.
Duplication, reviewed dead code and Any/unknown types are zero.

The independent HO concentrating fixture passed its precision comparison and
Rust full-band comparison. All local quality checks and the source-matched
[hosted Rust](hosted-ci.json) and [Python/repository](hosted-python-ci.json) checks
pass. P07 is complete; concentrating PDE qualification remains unresolved.

## Reports and reproduction

The [summary](summary.json) records the verified source scope and completed gates. The
[fixture recipes](recipes.md), [coefficient report](coefficient-fixture.json),
[scalar fixture report](scalar-fixtures.json),
[scalar arithmetic comparisons](scalar-arithmetic-comparison.json) and
[HO concentrating precision comparison](ho-oracle-comparison.json) preserve
independent numerical evidence. The [concentrating run log](concentrating.log)
records the actual Rust branch reservations and local ratios.
