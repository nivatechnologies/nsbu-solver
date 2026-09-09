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
There is no benchmark simulation CLI or qualified concentrating PDE window yet.
Pointwise and sampled direct-DFT checks do not establish trajectory convergence.

Part of NSBU Solver, Apache-2.0. No private Niva dependency or adapter.
