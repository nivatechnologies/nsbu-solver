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
```

Help/version and invalid-argument handling are implemented. The library provides validated domains, layouts, exact clocks, resource ledgers
and from-rest state allocation, spectral operators, transactional CM attempts, and
independent exact-v2 reference/forcing APIs. `cargo test --workspace` exercises these
library capabilities. There is still no numerical simulation command in the CLI.

## Intended runtime workflow, not yet executable

The planned binary is `nsbu`. Its interface will cover case export, preflight, integration, restart and experiment comparison. Exact flags and schemas are to be frozen and tested in P11; this table describes behavior, not a working command tutorial.

| Planned command family | Required behavior |
|---|---|
| `nsbu case write` | Write an exact, immutable case definition with its mathematical identity |
| `nsbu run` in dry-run mode | Report complete planned allocations, force coverage, exact interval, numerical method and capability refusals without integration |
| `nsbu run` | Evolve from the declared initial condition; record provenance, accepted/rejected attempts, diagnostics and checkpoint data |
| `nsbu resume` | Validate complete checkpoint identity and retain inherited numerical error and lineage |
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
bounded attempt API. The numerical CLI and convergence verifier remain later packages. See [P04 evidence](../evidence/p04/README.md).

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
endpoint does not qualify a PDE window. The public numerical CLI remains planned.
