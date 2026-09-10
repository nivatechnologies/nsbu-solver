# Physical refinements from independent trajectories

`smooth_experiment::physical` measures complete velocity, gradient, Hessian and
vorticity differences from the six independently evolved `SmoothFamily` branches.
It binds each report to their actual accepted clock and unchanged numerical
settings. A result requires all twenty quantity/pair comparisons to finish.
These are smooth diagnostic findings; no concentrating PDE window is accepted.

## Run the public walkthrough

```sh
cargo run --release -p nsbu-benchmarks --example smooth_refinement
```

The example uses the [existing branch schedule](EXPERIMENTS.md): grids 4/8/12,
macro steps 64/32/16 at quantum `2^-16`, and comparison ticks 0/64/128. All six
states start from rest. The diagnostic sample grid is `24³`; relative denominator
floors are fixed in advance at `1e-8, 1e-7, 1e-6, 1e-7`, in velocity/gradient/
Hessian/vorticity order. These are illustrative smooth diagnostic floors, not
independently qualified arithmetic floors or concentrating acceptance thresholds.

Each quantity reports N0/N1 and N1/N2 spatial differences, H0/H1 and H1/H2 temporal
differences, and a CM/HO method difference. RMS uses all ordered components and
divides by the number of physical points. Peak errors and pointwise relative
errors are also retained in the library report. No alignment, mean removal,
projection or fine-mode truncation is applied. See [field comparison
semantics](PHYSICAL_COMPARISONS.md).

## Admission and lifecycle

1. Create a `FamilyPlan` with immutable settings and an exact tested-time manifest.
2. Create `PhysicalFamilyPlan` with a sample layout, four positive finite floors,
   a finite attempt allowance and a cap covering the family plus this consumer.
3. Add the reservations of any other consumers before allocating them. The
   example also admits reconstruction and independent residual scratch.
4. Construct the family and physical workspace. After each successful family
   `advance`, call `physical.measure(&family)` once for that scheduled clock.
5. Store the returned complete report separately, within the caller's output
   budget. Read `next_time`, `remaining` and `charged_work` for the consumer ledger.

The physical consumer owns one reusable comparison workspace. It processes
components and pairs sequentially instead of retaining a Hessian at every point
for all six branches. Joint reservation includes the six owners, diagnostic
elements, fixed metadata and one returned record. Caller artifacts, additional
consumers and allocator overhead have separate budgets. Admission checks every
storage/work sum and product before allocation.

One complete attempt reserves **450 scalar inverse FFTs**: five pairs times
6 velocity, 18 gradient, 54 Hessian and 12 curl transforms. A second work counter
conservatively bounds weighted coefficient/sample visits outside FFT internals;
it is not a FLOP or wall-clock estimate. Both counts charge the full worst case
even when a malformed call fails before traversal. A failed call cannot replenish
its allowance or advance the required observation schedule. Exhausted calls are
refused without wrapping or changing the ledger.

The consumer requires the identical ordered time manifest, exact numeric policy
words (including signed zero), and the matching last successful family advance.
Every branch must have the required actual clock. A failed family is refused.
Public family access is read-only; these APIs cannot install analytical values or
imported checkpoint state. Reports have private construction and include the
actual clock and sample grid. They remain observations, not acceptance records.

## Verification and interpretation

```sh
cargo test -p nsbu-benchmarks --test physical_family -- --nocapture
cargo test -p nsbu-benchmarks --test allocation
cargo test -p nsbu-solver --test physical_comparison --test allocation
```

The actual six-branch test verifies every quantity and refinement pair, all state
digests, agreement of sampled velocity RMS with the independent Fourier L2
comparison, and decreasing temporal differences. Negative controls cover calls
before advance, repeated/stale samples, changed policy words/manifests, terminated
families, exhausted work, insufficient joint memory, bad floors and overflow.
Isolated allocator tests check admission refusal and no allocations during
measurement. The executable public example uses the same API.

At the active smooth sample times, temporal RMS differences decrease by roughly
16 between consecutive settings, consistent with fourth-order integration.
Very small spatial differences alone do not establish a supported error floor.
Zero differences at rest do not establish convergence. All quantities still need
their independent force/reference/arithmetic and physical sampling evidence.
The [pressure consumer](PRESSURE_REFINEMENTS.md) supplies separate scalar/gradient
findings. Regional concentrating reports, full frozen benchmark semantics,
accepted-state artifact binding and complete window production remain in progress.
