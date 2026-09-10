# Independent smooth comparison experiments

The `smooth_experiment` library module runs six owned `CyclicSine` trajectories
from exact rest and compares their actual fields. It also measures off-stage
reconstruction differences and a full-double-band PDE residual. These are
implemented diagnostic producers. They do not produce an accepted concentrating
window or a continuous-time error bound.

The separate [usage guide](USAGE.md) covers the installed single-run CLI.
`similarity-mms-v2` qualification still requires the full
[active implementation plan](../IMPLEMENTATION_PLAN.md).

## Run the example

From a public checkout with the pinned Rust toolchain:

```sh
cargo run --release -p nsbu-benchmarks --example smooth_refinement
```

The example uses the unit cube, viscosity 1, grids `4³, 8³, 12³`, and macro steps
`64, 32, 16` ticks at quantum `2^-16`. It compares fields at ticks `0, 64, 128`
and probes reconstruction at tick `127`. The physical endpoint is `1/512`.
Local absolute velocity/vorticity tolerances are `1e-2`, relative tolerances are
zero, and the advective guard is 0.3. These deliberately small smooth test settings
are fixed before execution; they are not concentrating-case acceptance tolerances.

Before constructing a trajectory, the example adds the complete family,
reconstruction and residual reservations and checks its 128 MiB cap. It prints
that storage allowance and finite provider/transform work. The reservation is
not measured resident memory: allocator and I/O overhead remain separate.
All numerical attempts and measurements use preallocated scratch.

The output includes full-fine-band spatial/time/method H1 differences, off-stage
velocity and physical-time derivative differences, and L2/H1/vorticity/divergence
residual norms. Norms use volume averages and the documented half-spectrum
multiplicities. Exact ticks accompany the measurements. Zero differences at rest
are expected and do not establish convergence.

## Branch schedule and ownership

`FamilyPlan::new` validates every run and the aggregate cap without allocating.
The tested-time manifest starts at rest, is strictly ordered, uses one exact clock
profile, ends at the configured endpoint, and must align to the largest step.
Three grids strictly increase; three positive macro steps strictly decrease and
nest by integer divisibility. The plan borrows the immutable time manifest.

| Slot | Method | Grid | Macro step | Purpose |
|---|---|---|---|---|
| 0 | Cox–Matthews | N0 | H2 | Coarse spatial branch |
| 1 | Cox–Matthews | N1 | H2 | Middle spatial branch |
| 2 | Cox–Matthews | N2 | H2 | Fine spatial/time baseline |
| 3 | Cox–Matthews | N2 | H0 | Coarse temporal branch |
| 4 | Cox–Matthews | N2 | H1 | Middle temporal branch |
| 5 | Hochbruck–Ostermann | N2 | H2 | Independent method branch |

`SmoothFamily::new` constructs six separately owned rest states, force/RHS
providers, attempt workspaces, observers and histories. `advance` evolves each
branch to the next declared time before forming its comparisons. The analytical
reference is never assigned to any branch. Public branch access is read-only;
callers cannot insert an imported state or replace accepted history in the family.

Comparisons are N0/N1, N1/N2, H0/H1, H1/H2 and CM/HO. Each pads the coarse field
onto the complete finer band and retains full, common-band and newly resolved
contributions. No alignment, rescaling or high-mode truncation is applied.

A local rejection or refusal ends the family. Already committed work on earlier
branches remains recorded; later branches are not advanced after the failure.
The family does not roll back accepted trajectories to conceal partial progress,
retry with a fresh allowance, or silently alter its settings. `BranchStopped`
identifies the branch and recorded outcome; further advances return `Terminated`.

## Accepted reconstruction and off-stage probes

Each reconstruction owner retains three accepted macro endpoints and derivatives
formed through its independent conservative operator. Pending diagnostic data
becomes accepted only after the physical/history transaction commits.

`ReconstructionWorkspace` reserves two vector interpolants and two derivative
fields on the finest grid. It compares temporal branches 3/4 and 4/2 at the same
exact probe. The three accepted histories must be nested, equally spaced and all
cover the probe. A time on any full/two-half CM/HO stage set is refused. For this
example, tick 124 is a stage time on the finest branch; tick 127 is off-stage.
Malformed probes consume one finite attempt and never alter physical history.

The rings cover their last two macro intervals. A probe outside those intervals
is refused; the workspace does not extrapolate. The current family schedule only
produces synchronized endpoint samples. A complete window sampler still needs
explicit retained geometry and coverage across the entire requested time set.

## Independently assembled residual

`ResidualWorkspace` owns interpolation buffers, a separate `CyclicSine` force
provider and conservative-product workspace, ten double-grid fields, and complete
work counters. At a valid probe it computes

```text
v_t + P div(v tensor v) - nu Delta(v) - P f
```

Here `v` and `v_t` come from accepted-history quintic Hermite reconstruction.
The force is evaluated at the exact probe on the doubled grid. The conservative
product is evaluated independently of the integration stages, and the result
retains the complete doubled band. No analytical field or stored stage residual
is substituted into this calculation.

A `ResidualSample` retains source geometry, accepted clocks and the run's origin.
Observing an imported run preserves `ExternalUnverified`. A checksum or a small
residual does not authenticate its history. Failed requests retain their complete
conservative work charge; diagnostic counters do not consume integration budgets.
Work units describe weighted visits and scalar transforms, not primitive FLOPs or
elapsed-time guarantees.

## Source and test map

| Source | Responsibility |
|---|---|
| [Family admission](../crates/nsbu-benchmarks/src/smooth_experiment/plan.rs) | Immutable schedule, six complete plans, checked storage/work totals |
| [Family runner](../crates/nsbu-benchmarks/src/smooth_experiment/mod.rs) | Independent evolution, terminal failures, full-band samples |
| [Reconstruction comparisons](../crates/nsbu-benchmarks/src/smooth_experiment/reconstruction.rs) | Bounded common-probe interpolation and geometry |
| [Residual producer](../crates/nsbu-benchmarks/src/smooth_experiment/residual.rs) | Independent force/products and full-double-band defect |
| [Residual admission](../crates/nsbu-benchmarks/src/smooth_experiment/residual/reservation.rs) | Complete fixed storage and worst-case work |
| [Example](../crates/nsbu-benchmarks/examples/smooth_refinement.rs) | Aggregate preflight and executable public walkthrough |

Focused development checks:

```sh
cargo test -p nsbu-benchmarks --test smooth_family --test offstage_residual
cargo test -p nsbu-benchmarks --test allocation
cargo test -p nsbu-benchmarks --example smooth_refinement
```

Tests exercise independent state buffers, synchronization, aggregate cap refusal,
terminal rejection, malformed/stage probes, spent budgets, and external-origin
preservation. The residual study evolves CM and HO from rest at three step sizes
and compares defects at one common genuine off-stage time. The isolated allocator
executable checks preflight refusal, construction bounds and allocation-free
integration, reconstruction and residual measurement.

## Remaining qualification work

These producers cover selected smooth space/time/method/reconstruction/residual
measurements. [Bounded checkpoint replay](CHECKPOINT_FORMAT.md#reproduce-an-imported-reconstruction-run-from-rest)
can independently reproduce a smooth checkpoint under the current executable; it
does not change an imported run or establish convergence.

The complete verifier must bind a frozen observable inventory,
full tested-time/sampling refinements, reference tracking and precision,
force sampling and precision, current-grid integrator arithmetic, pressure/local
errors, balances/quadrature, and any inherited transfer error. Concentrating core,
annulus and cutoff-collar coverage also remain mandatory. Missing evidence cannot
be assigned zero error. No package or PDE window becomes complete from this
example alone.
