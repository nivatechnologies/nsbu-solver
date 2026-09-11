# Runtime alpha

The runtime alpha is a public Rust library and command-line diagnostic for the
reviewed `similarity-mms-v2` manufactured-force problem. It is useful for
repeatable, bounded experiments from rest; it is not a qualified PDE result.
There are currently zero accepted concentrating windows. Qualification of larger grids, complete convergence studies, independent
arithmetic refinement, and further force optimization remain open work.

The solver evolves the coupled three-component velocity field from `u = 0`.
The exact target is `T_star = 1/128`; elapsed and remaining time use the exact tick
clock. Both Cox–Matthews (`cm`) and Hochbruck–Ostermann (`ho`) are available.
The default diagnostic profile is `N=4`, force sample grid `M=4`, fixed step
`128` ticks, tick quantum `2^-20`, endpoint `4096` ticks, at most `32`
attempts, absolute local tolerances `[1e-5, 1e-4]`, relative local tolerances
`[1e-5, 1e-5]`, and advective guard `0.3`. A run may select a finite
worker-thread allowance; resource preflight
charges the grid, provider, diagnostics, history, and worker reservation before
construction.

## Quickstart

From a clean checkout with Rust `1.94.0`:

```sh
cargo build --workspace --locked
cargo test --workspace --locked
cargo install --path crates/nsbu-cli --locked
cargo run -p nsbu-cli -- --help
cargo run -p nsbu-cli -- --version
cargo run --release -q -p nsbu-cli -- v2 --dry-run
```

The v2 CLI surface is exposed as `v2` and `resume-v2`. Use `nsbu v2 --help` and
`nsbu resume-v2 --help` for the supported option spellings. A normal run
must perform resource preflight before allocating, and a terminal refusal or
incomplete run exits nonzero. A successful exit reports a diagnostic status;
it does not qualify a window.

The reviewed profile can be selected with the currently shared option names:

```sh
nsbu v2 --grid 4 --force-grid 4 --method cm --step-ticks 128 \
  --tick-exponent -20 --target 4096 --attempt-cap 32 --workers 0 \
  --memory-cap 67108864
nsbu v2 --grid 4 --force-grid 4 --method ho --step-ticks 128 \
  --tick-exponent -20 --target 4096 --attempt-cap 32 --workers 0 \
  --memory-cap 67108864 --checkpoint run-v2.chk --checkpoint-after 16
nsbu resume-v2 --grid 4 --force-grid 4 --method ho --step-ticks 128 \
  --tick-exponent -20 --target 4096 --attempt-cap 32 --workers 0 \
  --memory-cap 67108864 --checkpoint run-v2.chk
```

The alpha fixes absolute tolerances `[1e-5, 1e-4]`, relative tolerances
`[1e-5, 1e-5]`, and advective guard `0.3`; these are part of the reviewed
profile rather than caller-provided switches.

The public library keeps the same ownership boundaries. `nsbu_benchmarks::v2_run`
exports `Plan`, `Settings`, and `Run`:

* `Plan::from_rest(settings, cap)` validates the exact clock, configuration,
  provider work, diagnostic storage, worker allowance, and total byte cap.
* `Run::from_rest(plan)` constructs a private zero state. Calling `step()` once
  records one bounded CM or HO attempt and returns its committed, rejected, or
  refused outcome.
* `Run::state()`, `history()`, `work()`, and `origin()` expose measurements and
  lineage without allowing an analytical reference field to replace the
  evolving state.

This is a complete small-grid construction of the reviewed default profile:

```rust
use nsbu_benchmarks::{runtime_force::ForceSettings, v2_run::{Plan, Run, Settings}};
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
    experiment::control::Configuration,
    integrators::{indicator::Tolerances, method::Method, trajectory::RunLimits},
};

let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
let settings = Settings {
    domain,
    force: ForceSettings { samples: Layout::new([4; 3]).unwrap(), workers: 0 },
    initial_clock: TickClock::from_rest(-20, 8192).unwrap(),
    configuration: Configuration {
        limits: RunLimits { endpoint: 4096, step_ticks: 128, maximum_attempts: 32 },
        method: Method::CoxMatthews,
        tolerances: Tolerances { absolute: [1e-5, 1e-4], relative: [1e-5, 1e-5] },
    },
    advective_limit: 0.3,
};
let plan = Plan::from_rest(settings, 64 * 1024 * 1024).unwrap();
let mut run = Run::from_rest(plan).unwrap();
let first_outcome = run.step().unwrap();
assert!(first_outcome.indicators().is_some());
```

The example performs one bounded attempt and leaves the run diagnostic-only;
callers must inspect the returned outcome and terminal report before treating
the trajectory as complete.

The external checkpoint path is deliberately conservative. A checkpoint is
accepted only when its profile matches the requested method, exact target and
step, tolerances, and finite allowances. Imported bytes retain an
`ExternalUnverified` origin. Resume is therefore a same-profile continuation
with an unverified external origin, not a reset from the reference field.
The encoded container carries a versioned header, canonical profile identity,
bounded payload length, and integrity hashes; the CLI preflights both the
maximum decoded reservation and the input buffer before reading checkpoint
bytes. A valid hash proves byte integrity only, not execution provenance. Restart word
equality is demonstrated within the tested build and execution environment. This
alpha format binds the numerical configuration and resource schema, not the
compiler binary or processor identity; cross-release and cross-platform bitwise
restart guarantees remain unsupported.

Reports include exact remaining time, all charged integration/observation work,
the last accepted balance terms, pending Simpson state, and any integrated
balance value. The CLI and library report terminal failures explicitly. A rejected attempt,
provider refusal, resource-limit refusal, malformed checkpoint, or incomplete
trajectory must remain visible and must not be presented as a completed
scientific run. See [scientific scope](SCIENTIFIC_SCOPE.md),
[checkpoint formats](CHECKPOINT_FORMAT.md), and [resources and errors](RESOURCES_AND_ERRORS.md).

The alpha does not claim an accepted concentrating endpoint or a singularity.
The analytical field and force are independent diagnostic inputs; neither is
assigned into the numerical state. Any future qualification release requires
the refinement, arithmetic, provenance, and accepted-window evidence specified
by the [active implementation plan](../IMPLEMENTATION_PLAN.md).
