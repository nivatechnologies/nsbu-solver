# NSBU Solver benchmarks

Independent Rust scalar reference and degree-four Taylor-jet implementations of
the exact `similarity-mms-v2` manufactured problem. `CASE_DEFINITION` preserves the
reviewed input bytes; its historical status text is not current project status.
The benchmark uses the unit periodic cube, viscosity one and exact target 1/128.

`BenchmarkTime` requires the exact dyadic target identity and separately converts
elapsed and remaining ticks. `scalar::evaluate` computes reference velocity and
raw pressure with explicit first derivatives. `fields::evaluate` independently
constructs implicit jets, force, force gradient and momentum diagnostics. Both
return structured errors; neither receives or changes an integrated state.
Raw pressure still needs its spatial mean removed for zero-gauge comparisons.

`V2Force::preflight` declares owned storage, a finite count of point assemblies and
root iterations, and three scalar transforms. Construct the provider with this
reservation and an explicit force-evaluation grid. The retained grid and sampling
grid are independent choices; resolving the latter requires a separate refinement
study. The provider performs no heap allocation during evaluation and supports
nonmonotone exact stage-time requests. A work unit is one bounded point assembly
or scalar root iteration, not a floating-point operation or wall-clock guarantee.
Fixed-size jets use call-stack storage, distinct from the provider's heap ledger.

The scalar root reports and jet residuals are floating arithmetic diagnostics,
not certified enclosures. The exponential coefficient majorant bounds the exact
formal polynomial, not rounding error. Binary64 force-gradient assembly can lose
absolute accuracy through cancellation: the preserved late pointwise sample has
an absolute residual of 8 in a nominally zero component. Differentiated momentum
term magnitudes expose this cancellation; they are not certified error bounds.

Development checks: `cargo test -p nsbu-benchmarks` from the public workspace.
Pointwise and sampled direct-DFT checks do not establish trajectory convergence.

## Bounded smooth run example

Run `cargo run -p nsbu-benchmarks --example smooth_from_rest` to construct a small owned
`CyclicSine` trajectory from rest. The example preflights fixed steps, source work, diagnostic
storage and history, then prints each recorded commit or bounded rejection/refusal. Its energy
and enstrophy values are measured diagnostics from accepted states. They are not a PDE-window
validation or a convergence claim.

`smooth_run::SmoothPlan::from_rest` performs allocation-free admission and exposes its frozen
configuration and resource plan. `SmoothRun::from_rest` creates the private state, RHS,
transaction scratch, observer and recorded history. The `observer_samples` argument must cover
the accepted measurements the run will need. `smooth_observer::BalanceObserver::limits` exposes
the diagnostic storage and total finite work before construction; `consumption` reports actual
charged calls, provider work and scalar transforms. Its observer trait bound uses per-sample
provider work; the separate transform count remains in this detailed diagnostic ledger.

Part of NSBU Solver, Apache-2.0. No private Niva dependency or adapter.

The `regions` module implements the reviewed core/annulus geometry, cutoff collar,
exact-tick startup classification, and bounded refined volume quadrature. Its
regional error collector preserves global measurements, exposes synchronized grid
coordinates, and bounds repeated root failures. An unsampled region is not an
empty region or a zero error. All masks and quadrature results remain floating
measurements requiring their own refinement; no window verifier is implied.

The `smooth_run` module exposes two observation profiles through one ownership
implementation: balance-only `SmoothRun` and `ReconstructedRun`, which retains
three accepted values and independently computed derivatives. Their trusted
snapshots preserve diagnostic work and the next transaction. The separate
`archive` and `reconstructed_archive` modules encode bounded external containers;
all imported owners remain `ExternalUnverified`. See the root checkpoint-format
guide for field layouts and the distinction between byte integrity and provenance.
