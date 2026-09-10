# Pressure from independently integrated states

`smooth_experiment::pressure` constructs physical mean-zero pressure and its
gradient from all six actual `SmoothFamily` trajectories. It reports the same
five spatial, temporal and method pairs as the [velocity refinement
consumer](PHYSICAL_REFINEMENTS.md). Both consumers require exact synchronized
accepted clocks, immutable numerical settings and a finite observation allowance.
These are smooth diagnostics; concentrating qualification remains incomplete.

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
