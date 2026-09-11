# Pressure from independently integrated states

`smooth_experiment::pressure` constructs physical mean-zero pressure and its
gradient from all six actual `SmoothFamily` trajectories. It reports the same
five spatial, temporal and method pairs as the [velocity refinement
consumer](PHYSICAL_REFINEMENTS.md). Both consumers require exact synchronized
accepted clocks, immutable numerical settings and a finite observation allowance.
These are smooth diagnostics; concentrating qualification remains incomplete.

## Exact-v2 pressure consumer

`v2_experiment::pressure` applies the same pressure construction to the six
actual exact-v2 branches. Its fixed inventory is pressure and full physical
pressure gradient across `(0,1)`, `(1,2)`, `(3,4)`, `(4,2)` and `(2,5)`. The
source is the finest retained V2 domain; each velocity is transferred without
discarding modes, while `ConservativeWorkspace` forms the complete quadratic
band on the doubled finest grid. A fresh original `V2Force` evaluates the
unprojected prescribed force at the exact accepted clock on that doubled grid.
The integration provider's fixed M=12 policy and this diagnostic force layout
are separate choices.

The report retains the actual clock, V2 family identity, source/force/sample
layouts and pressure/gradient floors. `PressureFamilyPlan` jointly preflights
the family plus provider, product scratch, comparison scratch and all temporary
buffers. One attempt charges the independent provider, ten conservative
assemblies and ten scalar/gradient comparisons: 133 scalar transforms in total
for V2. `measure` charges before its family checks, emits no partial report on
failure, advances only after all ten findings succeed, and leaves the borrowed
family states and histories unchanged. The pressure gauge is global mean-zero.

Focused commands are:

```sh
cargo test -p nsbu-benchmarks --lib v2_experiment::pressure
cargo test -p nsbu-benchmarks --test v2_pressure_family -- --nocapture
cargo test -p nsbu-benchmarks --test v2_pressure_allocation
```

The current integration log records pair (0,1) pressure RMS near
`2.9033e-13` and `1.1764e-13` for the two relevant observations, with gradient
RMS near `4.9015e-12` and `3.5812e-12`. Pair (4,2) includes an observed pressure
zero and gradient near `1.4044e-28`, while a stable independent oracle gives
`7.7445e-26` and `2.1294e-24`; common prescribed-force cancellation swamps this
tiny nonlinear difference. This is not qualified agreement or an arithmetic
floor. A separate private direct signed-mode force-only Poisson test validates
force sign, full-band use and the zero mode. Focused evidence is recorded under
`evidence/p09/v2-pressure`. Same-clock pair differences cancel the common
prescribed-force contribution, so they cannot qualify force sampling
independently. Complete reference-gauge, regional, arithmetic and
accepted-artifact qualification also remains pending.

## Public example

```sh
cargo run --release -p nsbu-benchmarks --example smooth_refinement
```

The example jointly admits all six from-rest owners, velocity/derivative and
pressure comparison workspaces, reconstruction and independent residual buffers
within 128 MiB. Its grids, steps and clocks are documented in the
[experiment guide](EXPERIMENTS.md). Pressure and pressure gradient use a `24³`
sample lattice and fixed relative denominator floors `1e-8` and `1e-7`.
The output includes RMS differences for every pair and each consumer's spent work.
Floors are illustrative diagnostic settings, not qualified error bounds.

## Independent construction and gauge

For each actual branch, the consumer pads a diagnostic copy of its velocity to
the finest family's complete strict band. This retains all coarse modes and
does not alter any branch or its history. An independent conservative workspace
then forms all six distinct products in `div(u tensor u)` on twice the finest
grid in every direction. It retains the full quadratic band, including pressure
modes absent from the velocity band.

The prescribed `CyclicSine` force is evaluated independently at the exact accepted
clock on that same doubled grid. It is reused across the branches because their
mathematical problem and time are identical. No integrator stage RHS or analytical
pressure is supplied. With `q = div(u tensor u) - f`, physical pressure satisfies

```text
Delta p = -div q
p_hat(k) = i k dot q_hat(k) / |k|², k != 0
p_hat(0) = 0
```

This is a common global mean-zero gauge. The comparison does not fit a constant,
subtract regional means, align fields or remove high-mode differences. Pressure
and its full three-component gradient are sampled through the same complete-field
comparison code used by other scalar diagnostics. RMS divides by physical points;
sampled absolute and pointwise relative peaks remain available in the result.

The exact smooth `CyclicSine` pressure is zero. Its independently integrated
states can have nonzero pressure errors. The producer retains those values and
never substitutes the known analytical zero for its numerical pressure.

## Resources and transactions

`PressureFamilyPlan::new` checks the family-plus-consumer reservation, complete
doubled-band sample layout, two positive finite floors and a finite call allowance
before allocation. `PressureFamilyWorkspace::new` constructs independent product,
force and comparison scratch plus three fine velocity and eight doubled-grid
complex arrays. Two pressure arrays suffice because the five pairs are processed
sequentially. Stored reports, other consumers and allocator overhead need separate
budgets. The plan includes fixed headers and one returned complete report.

Each attempt reserves one independent force evaluation, ten conservative pressure
assemblies and both pressure comparisons for all five pairs: **130 scalar
transforms** in total. Weighted coefficient/sample visits are tracked separately
from provider work and FFT counts. They are not a FLOP or wall-clock estimate.
Malformed and numerically failed attempts retain their full worst-case charge.

`measure(&family)` issues a report only after all ten quantity/pair findings
finish. It advances the observation schedule only on success. A changed manifest,
changed exact numeric policy words, stale/not-yet-accepted clock or terminated
family is refused. Exhaustion cannot replenish the budget. Reports are privately
constructed and record the actual clock, finest source geometry and sample grid.

## Verification and limits

```sh
cargo test -p nsbu-benchmarks --lib pressure
cargo test -p nsbu-benchmarks --test pressure_family -- --nocapture
cargo test -p nsbu-benchmarks --test allocation
```

Independent Taylor–Green coefficients check the sign, zero mode and pressure
outside the original velocity band, including known scalar/gradient RMS values.
A force-only fixture checks a pressure mode beyond the finest velocity band.
The actual six-trajectory study retains nonzero method errors, roughly 16-fold
temporal reductions and unchanged state digests. Negative controls cover geometry,
floor, cap, overflow, schedule, policy and terminal-family refusals. Allocation
instrumentation checks joint admission, construction and measurement.

These checks do not supply independent current-grid arithmetic, force/reference
precision, physical sampling refinements or complete benchmark protocol/accepted-
artifact binding for every pressure observable. Concentrating pressure also needs
independent analytical global-mean refinement and regional error reports. Small
spatial differences are not an independently supported floor, and sampled peaks
are not continuum enclosures. No concentrating PDE window is accepted.
