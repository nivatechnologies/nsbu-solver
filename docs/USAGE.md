# Usage and evidence

## Working commands in this checkout

From the repository root, with the [development environment](INSTALL.md) installed:

```sh
python tools/check_repository.py
python -m unittest discover -s tools/tests -v
python tools/verify_design.py --output work/design-checks.json
python -m json.tool benchmarks/similarity-mms-v2.json
```

The repository command checks packaging and immutable-input identities. The unit tests exercise the bootstrap's failure guards. The design runner recalculates selected design checks and writes JSON. The final command displays the exact case definition; it does not evaluate or integrate that case.

The report contains coefficient identities and floating-point junction checks, source-screening calculations, cutoff/endpoint geometry, an exact-clock design check, and resource reservations. `not_performed` includes Rust compilation, PDE integration, source-instance admission and formal proof building. Never convert `status: passed` into a claim that these unperformed tasks passed.

The commands have no integration timestep, grid or output-image options because they do not run a PDE solver. `--help` is available on each tool. Reports are emitted to stdout; `verify_design.py --output PATH` also writes the selected file. Relative report paths resolve against the checkout root. Reports inside the checkout must be `.json` files under `work/` or `evidence/`; the runner refuses to overwrite the preserved design evidence. A successful rerun matches that preserved scientific report before adding the current execution metadata.

## Working Rust package commands

```sh
cargo run -p nsbu-cli -- --help
cargo run -p nsbu-cli -- --version
cargo run -p nsbu-cli -- smooth --dry-run
cargo run -p nsbu-cli -- smooth --method cm
cargo run -p nsbu-cli -- smooth --method ho
cargo run -p nsbu-benchmarks --example smooth_from_rest
cargo run --release -p nsbu-benchmarks --example smooth_refinement
```

`smooth` evolves the built-in CyclicSine case independently from rest. The defaults
use an 8-cubed unit domain, viscosity 0.3, four 64-tick steps at tick exponent -16,
and a 64 MiB cap. JSON retains exact ticks as decimal strings, the immutable
profile, local tolerances, resource/work allowances and sampled energy/enstrophy.
Reports explicitly remain unqualified. Syntax errors exit 2; refusals or incomplete
runs exit 1; completed diagnostic runs and successful preflight exit 0.

`--dry-run` constructs only the checked admission plan. Allocation instrumentation
verifies that this admits no numerical-grid allocations, and admitted live and
imported run advances allocate no heap storage. The caller still budgets its
own I/O and allocator costs. `--help` lists the actual supported options.

The single-run library example uses viscosity 1 and a separate small four-step configuration.
The [six-trajectory experiment example](EXPERIMENTS.md) adds independent grid,
time-step and method comparisons, off-stage reconstruction and full-double-band
residual measurements under a separately checked aggregate cap.
Both are smooth verification profiles, distinct from `similarity-mms-v2`.
The CLI also saves and resumes balance-only smooth checkpoints. Concentrating
experiment commands and qualified restart provenance remain under implementation.

### Save and resume the smooth diagnostic

```bash
mkdir -p work
cargo run -p nsbu-cli -- smooth --method ho --checkpoint work/smooth-ho.bin --checkpoint-after 2 --dry-run
cargo run -p nsbu-cli -- smooth --method ho --checkpoint work/smooth-ho.bin --checkpoint-after 2
cargo run -p nsbu-cli -- resume --method ho --checkpoint work/smooth-ho.bin
```

Choose a new output path for each saved file: publication preserves existing files.
The checkpoint retains the original four-step endpoint and all work already spent.
`--checkpoint-after 0` saves the rest state; values beyond the configured number of
steps are refused before integration, including in dry-run. The dry-run includes
`checkpoint.maximum_bytes` in addition to the numerical resource ledger.

Resume uses the same grid, method, tick settings, endpoint and attempt allowance as
the saved run. Repeat nondefault options explicitly; supplied profile mismatches
are refused. `resume --dry-run` is not supported. A resumed result reports
`origin_status: external_unverified` even when this process wrote the source file.
Successful save reports `checkpoint_saved`; successful continuation reports
`completed`. Both remain `unqualified`. A SHA-256 match establishes byte integrity,
not an authenticated from-rest lineage. File checkpoint commands currently select
the balance-only `SmoothRun` profile; accepted reconstruction has separate trusted
in-memory snapshots and a bounded Rust library codec described below.

Files are bounded before input-sized allocation. Writes use a private temporary
file, synchronize its bytes, publish without replacing the destination, and
synchronize the parent directory. If publication succeeds but directory syncing
fails, exit code 1 and `checkpoint_published_durability_unconfirmed` explicitly
report that the destination already exists. Inspect that file before choosing a
retry path. These filesystem operations are tested on Linux; other filesystems
may refuse publication or durability confirmation.

## Intended runtime workflow, not yet executable

The remaining interface will cover concentrating case export, preflight, integration, qualified restart and experiment comparison. Exact flags and schemas are to be frozen and tested in P11; this table describes behavior, not a working command tutorial.

| Planned command family | Required behavior |
|---|---|
| `nsbu case write` | Write an exact, immutable case definition with its mathematical identity |
| `nsbu run` in dry-run mode | Report complete planned allocations, force coverage, exact interval, numerical method and capability refusals without integration |
| `nsbu run` | Evolve from the declared initial condition; record provenance, accepted/rejected attempts, diagnostics and checkpoint data |
| Qualified concentrating restart | Validate complete checkpoint identity and retain inherited numerical error and lineage; the existing `nsbu resume` supports only the smooth diagnostic |
| `nsbu experiment compare` | Apply the complete independent refinement protocol, full-band diagnostics and negative controls |
| `nsbu experiment extend` | Extend the same mathematical problem while preserving previous evidence and stopping at the last qualified interval |

`construction audit` is a separate optional mathematics-verification workflow. It is not required to install or use the generic solver. A viewer will read saved results after solver validation.

## First concentrating experiment

Use the exact [similarity-mms-v2 manifest](../benchmarks/similarity-mms-v2.json). It defines a unit periodic cube, `nu=1`, `h=1/8`, `T_star=1/128`, startup time `1/512`, inner cutoff `3/10`, and outer cutoff `21/50`. These are project benchmark inputs, not an admitted instance of the source construction.

Every comparison lineage begins at exact rest at time zero. The first target is `T_star/2`; further targets follow `t_k = T_star(1 - 2^(-k))`. Force and reference evaluators are separate. The integrator may use the prescribed force but never replace its state with the analytical reference.

First run a resource preflight, then diagnostic pilots, then freeze tolerances and run the required space/time/force/reference/arithmetic comparison families. Do not initialize at a later exact field to shorten the calculation while retaining a from-rest label. Every claimed endpoint needs the complete evidence family, including full-band errors and inherited history.

At `512^3`, the twelve-cell screen reaches three endpoint indices on that individual grid. This does not establish three converged intervals. Coarser comparison grids and measured errors may shorten the qualified frontier. The base memory reservation alone is about 79.68 GiB, with additional costs explicitly required.

## Reports required from the future runtime

Every run must identify the mathematical problem, reference/force artifacts, execution profile, exact start/end times, resource plan, method, and checkpoint lineage. A window report must distinguish diagnostic-only, accepted and rejected intervals and record the limiting channel. Sampled maxima and empirical reconstruction errors cannot be presented as rigorous bounds.

A useful published concentrating result contains the immutable case, from-rest branches, frozen comparison protocol, space/time/input/arithmetic refinements, pressure and balance diagnostics, invalidation records, accepted/rejected endpoints and the last qualified time. It does not claim a singularity proof from finite numerical samples. The [implementation plan](../IMPLEMENTATION_PLAN.md) specifies the required negative controls.

## Independent reference commands now available

P00B is complete. From the repository root, run:

```sh
python -m unittest discover -s reference/tests -v
python -m reference.verify_fields > work/reference-fields.json
python -m reference.verify_steps > work/reference-steps.json
python -m reference.verify_trajectory > work/reference-trajectory.json
python -m reference.verify_sampling > work/reference-sampling.json
```

Create `work/` first if the bootstrap runner has not created it. These commands
validate pointwise v2 evaluations and N=4 smooth semidiscrete arithmetic fixtures.
The step fixture starts from rest; its force is a separate smooth case. It does
not qualify a concentrating PDE window. See [reference scope](../reference/README.md).

The trajectory study reports measured temporal orders for a separate smooth MMS.
The sampling study uses the exact v2 force, but performs no integration and grants
no spatial qualification. Larger direct-DFT and sampling studies can take many
minutes. Each numerical study reserves its diagnostic storage before allocation;
these Python reservations are not hard allocator bounds for the future runtime.

## Executed concentrating diagnostic

`python -m reference.pilot` runs the N=4, 80-digit CM pilot from exact rest to
1/256. This is an allocating Python diagnostic and can take about twenty minutes.
The [saved CM/HO reports](../evidence/p00c/README.md) include the final spectra,
resource reservations, force-sampling records and large reference-tracking errors.
Neither method qualifies a concentrating window.

## Rust numerical development checks

The library implements CM steps, exact-interval full/two-half attempts and
transactional commits. These commands exercise independent smooth step fixtures
and allocation/rollback contracts; they do not qualify a concentrating trajectory:

```sh
cargo test -p nsbu-solver --test cm_dft
cargo test -p nsbu-solver --test transaction
cargo test -p nsbu-solver --test attempt_allocation
```

Force implementations declare their own complete work/storage limits. Undeclared
callbacks may use the standalone research step kernel, but are refused by the
bounded attempt API. The concentrating CLI and convergence verifier remain later packages. See [P04 evidence](../evidence/p04/README.md).

## Independent Rust exact-v2 evaluation

`cargo test -p nsbu-benchmarks` exercises scalar and jet evaluations, exact clock
separation, high-precision field/root/DFT fixtures, bounded provider admission, and
allocation-free force requests. The crate [API overview](../crates/nsbu-benchmarks/README.md)
describes its storage and arithmetic contracts. P06 extends this suite with smooth
refinement studies and an N=4 concentrating trajectory compared with independent
80/120-digit direct-DFT evolution. The concentrating test evolves from rest and
commits only integrated fine-step states; no reference state is assigned. Its
coarse spatial and force sampling errors remain unresolved. Force-sampling
convergence and cancellation-sensitive arithmetic accuracy still require the
later experiment studies.

The focused development commands are:

```sh
cargo test -p nsbu-benchmarks --test smooth
cargo test -p nsbu-benchmarks --test refinement -- --nocapture
cargo test -p nsbu-benchmarks --test concentrating -- --nocapture
```

P06 passed all local and hosted quality checks. Reaching the diagnostic
endpoint does not qualify a PDE window. The concentrating CLI remains planned.


## Independent HO development checks

The library now provides `HoCoefficients`, `HoWorkspace` and
`Method::HochbruckOstermann`. The existing default attempt constructors select CM.
For a bounded HO attempt, reserve
`AttemptWorkspace::reservation_with_method(domain, method)` in the resource plan
before calling `AttemptWorkspace::new_with_method(plan, method)`. HO requires
fifteen RHS calls per full/two-half attempt; CM requires twelve. Both use the same
single-use transactional commit protocol.

```sh
cargo test -p nsbu-solver --test ho_coefficients
cargo test -p nsbu-solver --test ho_kernel -- --nocapture
cargo test -p nsbu-solver --test ho_dft
cargo test -p nsbu-solver --test attempt_allocation
cargo test -p nsbu-benchmarks --test concentrating -- --nocapture
```

The concentrating test compares both independently evolved methods with their
80/120-digit direct-DFT fixtures and with each other. It can take several minutes
in a debug/instrumented build. These remain coarse diagnostics with no accepted
PDE window. [P07 evidence](../evidence/p07/README.md) records order reduction,
arithmetic comparisons and the passed hosted verification. The numerical
CLI and window verifier remain planned.


## Independent diagnostics development checks (P08 in progress)

The library's `diagnostics` module now provides `ComparisonPlan`,
`ConservativeWorkspace`, `HermiteWeights`, `ResidualPlan`, balance measurements,
and Simpson quadrature. Conservative diagnostics require their separate double-grid
reservation plus caller buffers and allocator overhead. Primary comparisons retain
all fine modes and preserve mean differences. Hermite inputs are accepted-history
values and derivatives supplied by the experiment, with exact matching clocks.

```sh
cargo test -p nsbu-solver --test band_comparison --test force_aliasing
cargo test -p nsbu-solver --test conservative --test hermite --test residual --test quadrature
cargo test -p nsbu-benchmarks --test diagnostic_history -- --nocapture
```

These commands work now. The [source-matched evidence](../evidence/p08/core/README.md)
records analytic negative controls and independently evolved smooth histories.
Sampled residuals and shrinking-interval maxima are not continuous error bounds.
The complete P08 window review, concentrating CLI, and qualified concentrating
results remain incomplete. No reference field is assigned to an integrated state.

Sampled reporting is also implemented. `SamplingWorkspace` separately preflights
velocity/vorticity transforms and returns sampled maxima with unaligned physical
positions. `TailPlan` measures overlapping directional Fourier tails; spectrum
integrity inspection reports defects without repairing the input. `ErrorAccumulator`
retains absolute errors and a declared positive relative floor. Missing samples
remain `NoSamples`, never zero error.

The benchmark's `regions` module supplies geometric masks, independently refined
volume fractions and `RegionalErrors`. A collector exposes each next grid point
for synchronized independent evaluations and preserves both global and regional
errors. Failed classifications consume its finite root-work allowance. Sampling
absence is distinct from the geometric `RegionEmpty` status. The caller must still
establish sampling and arithmetic resolution.

```sh
cargo test -p nsbu-solver --test sampling --test tails --test spectrum_integrity --test local_errors
cargo test -p nsbu-benchmarks --test regions --test regional_errors
cargo test -p nsbu-benchmarks --test balance_history -- --nocapture
```

The balance study uses actual accepted smooth histories from rest, independently
formed physical pressure and separately refined quadrature. It checks both CM and
HO against smooth reference quantities; it is not a concentrating-window result.

The [reporting evidence](../evidence/p08/reporting/README.md) records measured scope,
independent fixtures, numerical limits and the current quality results.


## Bounded measurement review development checks

The library's `verification` module reviews explicitly supplied measurements:

```bash
cargo test -p nsbu-solver --test refinement_rules --test tested_times --test reconstruction_policy --test reconstruction_samples --test verification_budget --test measurement_review
```

These tests exercise separate error-channel rules, conservative budget allocation,
three nested exact time sets, off-stage probes and bounded streaming observations.
A numerical pass is `ReadyForLineageReview`; it does not certify input provenance,
current-grid accuracy, an accepted PDE window or an enclosure. The complete
experiment verifier and concentrating CLI remain under implementation. See
[window measurement evidence](../evidence/p08/window/README.md).


## Lineage and physical-image development checks

```bash
cargo test -p nsbu-solver --test lineage_registry --test physical_image
cargo test -p nsbu-solver --test allocation
```

The public `lineage` module provides bounded ancestry declarations, transitive
force invalidation and an in-memory `PhysicalImage` of a live state. Image tests
reproduce the next accepted/rejected CM and HO attempts with fresh scratch. An
image alone is not a complete checkpoint; the smooth CLI composes additional
controller, balance and work records. See [foundation evidence](../evidence/p09/foundations/README.md).


## Recorded-step and balance-history development checks

```bash
cargo test -p nsbu-solver --test controller --test balance_history --test balanced_commit --test recorded_step
cargo test -p nsbu-solver --test recorded_allocation
```

The library's `experiment::runner::recorded_step` combines a bounded core attempt,
a read-only proposal observer, compensated balance history and a preallocated
outcome log. Inspect the returned `Outcome`: `Ok` may contain a terminal refusal.
The separate controller and history components are restartable, but they are not
a complete coherent checkpoint or a serialized format by themselves. The smooth
owner supplies the separately documented container. See [actual evidence](../evidence/p09/recorded/README.md).


## Artifact integrity and raw-history replay checks

```bash
cargo test -p nsbu-solver --test checkpoint_artifacts --test history_replay --test recorded_step
```

`checkpoint::artifacts` verifies exact content bytes and a bounded canonical
catalog. `RunHistory::replay` checks a raw measured attempt log and reconstructs
its controller and compensated balances. These APIs do not authenticate physical
provenance or provide complete checkpoint file read/write commands. See
[artifact/replay evidence](../evidence/p09/artifacts/README.md).

## Owned smooth runs and binary components

`nsbu_benchmarks::smooth_run::SmoothPlan` preflights the complete supported profile;
`SmoothRun` owns state, private integrator/provider/observer scratch, raw history and
per-attempt work. It exposes no mutable reference-assignment path. Trusted in-memory
snapshots retain the owner origin and restore fresh scratch.

The [experimental binary formats](CHECKPOINT_FORMAT.md) preserve physical bits,
exact clocks, configuration, controller/balance history and work. Importing the
smooth-owner format always creates `ExternalUnverified` data. Its explicit diagnostic
continuation retains that status through later snapshots and exports. A valid
checksum and consistent history do not authenticate physical provenance.

The lineage event codec preserves interleaved append/invalidation attempts, including
failures and spent allowances. Coarse-to-fine prolongation retains past error: an
actual forced-shear test compares continuation against independently evolved direct
fine trajectories and detects the missing inherited high mode.

`ReconstructedPlan` and `ReconstructedRun` select the independent reconstruction
observer through the same owned integrator and transaction implementation. The
observer evaluates the actual rest node and each proposed endpoint using its own
conservative double-grid RHS, then publishes accepted nodes only after the physical
and raw-history commits. Three equally spaced accepted macro endpoints support
Hermite value/derivative probes between them. Rejected or refused steps preserve
the accepted ring and retain all spent work.

`ReconstructedRun::snapshot` includes the accepted values, independent derivatives,
clock/epoch metadata and diagnostic work counters. Restore creates fresh numerical
scratch and preserves the next accepted or rejected attempt. Query reconstruction
through `run.observer().reconstruct(...)` with caller-owned output slices. This API
does not allocate during interpolation or later attempts. Reconstruction samples
are empirical diagnostics. The `smooth_run::reconstructed_archive` library API writes and reads the separate
`NSBURC01` container, preserving this history through unverified external imports.
Its `maximum_encoded_len`, `encoded_len`, `write` and `read` functions require
explicit caller buffers and caps. See the [format guide](CHECKPOINT_FORMAT.md)
for layout, recovery and origin semantics. The CLI still saves balance-only
checkpoints. Complete experiment provenance remains in progress; P09 is incomplete.

Exercise the actual next-attempt and corruption checks with:

```bash
cargo test -p nsbu-benchmarks --test reconstruction_archive --test reconstructed_owner_archive
cargo test -p nsbu-benchmarks --test allocation
```

## Library checkpoint replay

After decoding and continuing a reconstruction archive as an unverified owner,
`smooth_run::replay::ReplayPlan::new(&run, maximum_attempts, cap)` checks simultaneous
storage and the complete recorded attempt budget. `execute()` independently evolves
a fresh rest state and requires equality of canonical physical/history/work/node
bytes. The original remains unchanged. Read [the replay contract](CHECKPOINT_FORMAT.md#reproduce-an-imported-reconstruction-run-from-rest)
for origin handling, work bounds and its execution-profile limitations. This is a
Rust library API; the installed file CLI does not invoke numerical replay yet.

## Independent smooth arithmetic

The [arithmetic walkthrough](ARITHMETIC_STUDY.md) exports actual Rust state and
exact stage-force bits, evolves independent 80/120-digit direct-DFT trajectories,
and compares their complete same-grid fields. `python -m reference.verify_cyclic
--n 12 --method HO --precision 120 --dry-run` performs reference resource admission.
The full guide documents fixed-input validation, command exit codes, the
five separate differences and the distinction between input hashes and provenance.
These diagnostic commands do not produce accepted PDE windows.

## Physical sampling refinement

```sh
cargo run --release -p nsbu-benchmarks --example smooth_sampling -- --dry-run
cargo run --release -p nsbu-benchmarks --example smooth_sampling
```

The [sampling guide](SAMPLING_REFINEMENTS.md) documents complete velocity tensors,
vorticity, pressure and pressure-gradient comparisons on M=24/32/48 grids at
unchanged actual accepted clocks. The example preserves all pairwise statistics
and reports their sampling changes, with a joint memory cap and finite work.
Sampled peaks and near-zero changes do not establish continuum error bounds.

## Streaming reconstructed probes

```sh
cargo run --release -p nsbu-benchmarks --example smooth_probes -- --dry-run
cargo run --release -p nsbu-benchmarks --example smooth_probes
```

The [accepted-history guide](RECONSTRUCTED_PROBES.md) describes the exact nested
time manifests, two-step initial lookahead and read-only interpolation fields.
The example emits early and late probe findings from actual independent histories
and records both physical probe time and each owner's current state time. Its
interpolated fields are diagnostic; full all-observable window qualification
remains in progress.
