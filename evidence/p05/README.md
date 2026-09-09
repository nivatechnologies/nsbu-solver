# P05 independent exact-v2 Rust fields and forcing

The public benchmark crate now contains separate explicit scalar reference formulas
and degree-four, four-variable Taylor jets for the reviewed v2 definition. The
implicit root uses bounded safeguarded scalar Newton updates followed by three
formal Newton corrections. No finite differences construct the force. Velocity,
raw pressure, force, spatial force derivatives, momentum terms and floating root
residuals are available for verification. None can initialize or reset an evolving
state. The embedded case remains byte-identical to the reviewed input.

The exact clock requires target 1/128 and converts elapsed and remaining counts
independently. Tests include the smallest positive elapsed and remaining tick at
exponent -134. At the last tick, rounded elapsed equals the target but remaining
stays positive. Periodic centering preserves already-centered coordinates, avoiding
cancellation when wrapping tiny negative inputs. Scalar and jet cutoff/startup
branches, mixed derivatives, axis limits and pressure agreement are checked.

## Numerical evidence

Nine independent 120-digit pointwise fixtures exercise rest, the axis, startup,
cutoff edges/collar and a late concentrating point. Three independent implicit-jet
fixtures compare all 70 coefficients. Their 80/120-digit scaled change is at most
1.401e-75. The N=4 sampled-force direct-DFT fixture changes by at most 2.094e-79
between 80 and 120 digits. Rust coefficients agree within the explicit test budget.
These are pointwise and sampled-coefficient checks, not spatial convergence tests.

[pointwise.log](pointwise.log) records actual binary64 discrepancies. Force scaled
error is at most 4.547e-15 across the nine samples. The late force-gradient sample
has maximum absolute discrepancy 32; a nominally zero component has discrepancy 8.
The ordinary componentwise relative/absolute gradient test therefore does not pass
there. The test separately checks the residual against differentiated momentum
term magnitudes and reports the absolute failure of accuracy. This cancellation
is unqualified; the diagnostic scale is not a certified error bound.

The late root-jet equation's maximum raw coefficient residual is 1.953125e-3,
while comparison of the root coefficients with the independent high-precision
oracle passes the 2e-13 scaled coefficient budget. Raw coefficients represent
mixed derivatives of different dimensions and magnitudes. Neither this residual
nor the scalar root estimate is a certified enclosure or a trajectory error bound.
The exponential majorant bounds exact formal-polynomial magnitudes, not roundoff.

## Provider resources and interfaces

`V2Force` rejects a domain other than the unit cube with viscosity one. The caller
chooses a separate evaluation grid no smaller than the retained grid. Preflight
accounts for eleven owned allocations, object headers and an explicit allocation
overhead allowance. One evaluation uses at most 129 work units per point and
three scalar FFTs. A unit means one fixed-degree assembly or one scalar root
iteration; it is not a primitive-operation count or wall-clock bound. Fixed-size
jet temporaries consume call-stack storage, distinct from the owned heap ledger.

The allocation executable measures construction and repeated nonmonotone requests.
Owned buffers fit the declared reservation and evaluations allocate, reallocate
and deallocate zero times. Full-band transfer omits strict Nyquist planes without
rescaling. The force provider cannot read the evolved velocity or access reference
assignment. Force sampling and arithmetic refinement remain separate later studies.

## Quality and architecture review

[summary.json](summary.json) binds the measured source and fixtures by SHA-256.
All source and tests remain included in line/branch, complexity and duplication
scope. Per-function cyclomatic and cognitive complexity, per-function/file
Halstead difficulty, physical file length, CRAP, mutation outcomes and dependency
licenses are recorded separately. LLVM regions and inactive instantiations are
reported without claiming complete coverage of those different measures.

Responsibilities are separated into clock conversion, root solving, polynomial
algebra, explicit scalar fields, jet assembly and sampled forcing. The solver
accepts the narrow prescribed-force contract; benchmark-specific dependencies
point toward the solver interface. Providers declare immutable costs and obey
bounded evaluation semantics. Reference queries remain a separate interface.
Clippy's denied warnings and review found no dead code or dynamic type escape;
the duplicate detector finds no cloned source blocks. No private dependency or
adapter was added. The final full-workspace mutation run tested 1,356 replacements: 1,248 were
caught and 108 could not compile, with zero survivors or timeouts. All local
checks pass; package completion still requires hosted verification.

A reused Cargo temporary registry initially selected the P01 facade of the same
development version during package verification. Repeating packaging with a fresh
target verified all three current archives. CI now uses a fresh temporary target;
no registry publication occurred. Cargo's [packaging contract](https://doc.rust-lang.org/cargo/commands/cargo-package.html)
explains dependency normalization; the observed stale local extraction is preserved
in the work log and the successful fresh run in [package.log](package.log).

No Rust concentrating trajectory, accepted PDE window, numerical CLI, checkpoint
or viewer is supplied by P05. Those remain later packages in the active plan.
