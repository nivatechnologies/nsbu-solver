# Exact-v2 reconstructed balance quadrature

`v2_experiment::probes::balances::V2Balances` owns six independent exact-v2
balance observers and consumes immutable fields from one admitted `ProbeFamily`.
Each report retains the complete reconstruction record, family identity, actual
accepted-node origins and six instantaneous balance samples.  Unlike the
residual consumer, balance sampling may occur at rest, accepted endpoints and
integrator stage clocks; it still binds the exact ordered probe manifest.

Each branch computes conservative balance terms on its own doubled diagnostic
band (`N=4/8/12` to `2N=8/16/24`).  Every observer evaluates a fresh original
exact-v2 force on the same configured `M=24` input grid used by the reconstructed
trajectory observer.  Admission verifies that fixed grid covers the finest
diagnostic band and reserves all six observers, provider work, report metadata,
attempts and simultaneous `ProbeFamily` storage before allocation.

`V2BalanceQuadrature` wraps the balance consumer with three private
`BalanceHistory` levels.  The focused profile uses exact clocks
`0/64/128`, `0/32/64/96/128`, and `0/16/.../128`.  Admission checks membership,
strict nesting, odd sample counts, equal Simpson half-panels, decreasing maximum
spans, complete history/update work and joint storage.  Every affected history
is copied and updated privately; counts and a complete three-level report commit
together.  A failed child, binding or history update terminates the owner and
exposes no partial report or partially updated count.

The numerical fixture freshly evaluates the M24 force and reconstructs the
conservative balance path outside the wrapper at clock 96.  A force-omission
control must separate at least one forcing observable.  A hand-weighted Simpson
reduction of every stored actual balance sample checks all eighteen reported
integrals without calling the wrapper or `simpson`.  A separate quadratic
polynomial fixture checks the reused `BalanceHistory` Simpson arithmetic against
an analytical integral.

These are sampled balance and quadrature diagnostics.  They do not bound the
continuous-time balance, qualify force coverage, assemble residual or pressure
evidence, or accept a concentrating window.  P09 and P10 remain incomplete.
