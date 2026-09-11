# NSBU Solver

A standalone Rust project for incompressible, three-dimensional Navier–Stokes simulation and carefully qualified concentrating-flow experiments.

**Status: runtime alpha ready; zero accepted PDE convergence windows.** The Rust library and CLI run bounded smooth and exact-v2 CM/HO trajectories from rest, including checkpoint/resume. All local runtime, quality and clean-install gates pass; publication requires hosted CI for the exact release commit. Scientific qualification remains in progress. See the [runtime alpha guide](docs/RUNTIME_ALPHA.md) and [validation evidence](evidence/runtime-alpha/README.md).

The library implements spectral operators, CM/HO integration, exact-v2 forcing, bounded transactional attempts, and independent diagnostics. The CLI provides bounded smooth and v2/resume-v2 commands; all external resumes retain an unverified origin. This checkout also preserves the reviewed specification, benchmark manifest, mathematical checks, and active implementation plan.

The NSBU Solver library evolves all three velocity components on a periodic three-dimensional domain, with fixed positive viscosity and a prescribed force:

```text
partial_t u + (u dot grad)u = -grad p + nu Laplacian(u) + f(x,t)
div u = 0,  nu > 0
```

The library uses Fourier pseudospectral discretization, 3/2 padding for quadratic products, incompressibility projection, and exponential Runge–Kutta integration. Bounded attempts, the exact integer tick clock, transactional state commits, and resource preflight are implemented. Complete same-profile runtime checkpoint/resume is implemented for smooth and exact-v2 diagnostics. Scientific checkpoint provenance and the concentrating window-convergence verifier remain in progress.

## What the first experiment means

The first concentrating case is **`similarity-mms-v2`**, a manufactured prescribed-force problem. Each comparison trajectory starts from rest and evolves independently. The analytical field is available to the verifier, never as a replacement for the evolving state.

This case tests the solver's ability to track a separately specified concentrating field over successively closer finite intervals. It does not reproduce the source paper's annular pulse construction, establish a smooth forcing extension through the target time, or prove finite-time blow-up. Literal reproduction of the source construction is outside the initial release.

The explicit slow-mesh strategy is `FeasibilityExcluded` under the documented resource policy. Feasibility across unspecified representations remains `FeasibilityUnestablished`. The optional construction compiler is limited to mathematics verification; an averaged-stress surrogate remains a separate, deferred model. See the [scientific scope](docs/SCIENTIFIC_SCOPE.md).

## Get started today

Requirements: Python 3.12 and the development dependencies below. Git is needed only when obtaining a checkout by cloning. Rust is not required to run the existing design checks.

```sh
git clone https://github.com/nivatechnologies/nsbu-solver.git
cd nsbu-solver
python3 -m venv .venv
. .venv/bin/activate
python -m pip install -r requirements-dev.txt
python tools/check_repository.py
python -m unittest discover -s tools/tests -v
python tools/verify_design.py --output work/design-checks.json
```

When using the complete source ZIP, extract it with its directory structure intact and start from the extracted `nsbu-solver` directory, omitting the clone command. The archive includes hidden paths such as `.github/workflows/checks.yml`, `.gitignore` and `.gitattributes`; selecting only visible files can omit required bootstrap inputs.

The repository check verifies the frozen inputs, mathematical problem identity, local documentation links, and public-package boundaries. The unit tests exercise missing-file, modified-input and unsafe-output failures. The design runner reruns selected algebra, coefficient, clock, geometry, resource, and source-screening checks and compares their report with the preserved evidence before adding execution metadata. Its `status: passed` means those checks passed; it does not mean a PDE simulation passed.

See [installation](docs/INSTALL.md) for setup, [runtime alpha](docs/RUNTIME_ALPHA.md) for the v2 profile, and [usage](docs/USAGE.md) for report interpretation. The Rust workspace builds with Rust 1.94.0. Every command performs bounded admission and reports diagnostic status; none qualifies the concentrating benchmark.

The [independent Python reference](reference/README.md) now provides pointwise v2
scalar/jet evaluations, direct-DFT fixtures, and N=4 smooth trajectories evolved
from rest with temporal and 80/120-digit arithmetic refinements. P00B and its Python quality gates pass. The [coarse concentrating diagnostic](evidence/p00c/README.md) also reaches the first endpoint
from rest with both methods, with large tracking errors and unresolved spatial
sampling. No qualified concentrating PDE trajectory is established. The Rust
[benchmark library](crates/nsbu-benchmarks/README.md) now provides independent v2
scalar/jet evaluations and a bounded sampled force provider; its final package
verification passed locally and in hosted CI. The [first Rust concentrating diagnostic](evidence/p06/README.md)
has also reached 1/256 from rest on N=4, with spatial/force resolution unresolved.

## Implementation path

| Milestone | Result |
|---|---|
| Bootstrap | Public specification, Apache-2.0 license, checks, and contributor documentation |
| Independent reference | Scalar/jet evaluators and high-precision direct-DFT fixtures |
| Rust numerical core | Domains, layouts, FFT operators, projection, pressure, and bounded ETDRK4 steps |
| Independent verification | Benchmark force evaluator, second time integrator, full-band diagnostics, restart and lineage tests |
| Qualified experiments | From-rest refinement families over successively closer finite intervals |
| Public runtime release | Tested library and CLI installation, examples, evidence artifacts, then a read-only viewer |

The [implementation plan](IMPLEMENTATION_PLAN.md) supplies dependencies, concrete work packages, stop conditions, and release gates. A generic runtime release and a qualified concentrating-results release have separate evidence requirements.

## Resources

These values are conservative design reservations, excluding FFT plans, provider scratch, diagnostic grids, I/O, and allocator overhead. They are not measured usage or convergence results.

| Grid | Base reservation | Maximum endpoint index under the 12-cells-per-peak-radius screen |
|---|---:|---:|
| `128^3` | 1.25336 GiB | 0 |
| `256^3` | 9.98218 GiB | 1 |
| `512^3` | 79.67871 GiB | 3 |
| `1024^3` | 636.71484 GiB | 5 |

Here `t_k = T_star * (1 - 2^(-k))`, with `T_star = 1/128`. The screen is a heuristic, not an acceptance test. Coarser comparison branches can reduce the qualified frontier. A `128^3` run would be a diagnostic pilot under this policy.

## Project documents

| Document | Purpose |
|---|---|
| [Implementation plan](IMPLEMENTATION_PLAN.md) | Active NSBU work sequence and release criteria |
| [Complete numerical design](docs/design/COMPLETE_DESIGN.md) | Frozen revision 0.7 engineering baseline |
| [Benchmark manifest](benchmarks/similarity-mms-v2.json) | Exact rational inputs and mathematical problem identity |
| [Scientific scope](docs/SCIENTIFIC_SCOPE.md) | Meaning and limits of results |
| [Provenance and adopted decisions](docs/PROVENANCE.md) | Historical naming, baseline hashes, licensing overlay |
| [Architecture](docs/ARCHITECTURE.md) | Module responsibilities, ownership and transaction flow |
| [Smooth experiment walkthrough](docs/EXPERIMENTS.md) | Six independent trajectories, off-stage reconstruction and PDE residual samples |
| [Parallel force sampling](docs/PARALLEL_FORCE.md) | Persistent worker ownership, exact coefficient comparisons and resource limits |
| [Force evaluation](docs/FORCE_EVALUATION.md) | Exact-v2 axial root reuse, bounded resources and coefficient-preservation checks |
| [Arithmetic study](docs/ARITHMETIC_STUDY.md) | Same-grid Rust/direct-DFT comparisons with separate prescribed-force effects |
| [Derived-field arithmetic](docs/DERIVED_ARITHMETIC.md) | Complete velocity tensors, vorticity and physical pressure compared at 80/120 digits |
| [Physical-reduction arithmetic](docs/REDUCTION_ARITHMETIC.md) | Production tensor statistics compared with independent 80/120-digit exact-word reductions |
| [Numerical conventions](docs/NUMERICAL_CONVENTIONS.md) | Fourier, pressure, norms and exact-time contracts |
| [Checkpoint formats](docs/CHECKPOINT_FORMAT.md) | Experimental byte formats, caps and import-origin boundaries |
| [Numerical protocol identity](docs/PROTOCOL_FORMAT.md) | Canonical tolerance rules, exact tested times and reconstruction geometry |
| [Physical derivative diagnostics](docs/DERIVATIVE_DIAGNOSTICS.md) | Full-band scalar derivatives, v2 reference tensors and regional errors |
| [Physical-field comparisons](docs/PHYSICAL_COMPARISONS.md) | Complete scalar/vector/tensor comparisons and v2 regional aggregation |
| [Physical refinement families](docs/PHYSICAL_REFINEMENTS.md) | Actual six-trajectory comparisons, exact clocks and finite diagnostic budgets |
| [Reference pressure gauge](docs/PRESSURE_REFERENCE.md) | Exact-v2 global mean with independent bounded quadrature/arithmetic refinements |
| [Pressure refinements](docs/PRESSURE_REFINEMENTS.md) | Independent full-band physical pressure from actual accepted states |
| [Sampling refinements](docs/SAMPLING_REFINEMENTS.md) | Three physical sample grids, complete tensors/pressure and preserved peak sensitivity |
| [Accepted-history probes](docs/RECONSTRUCTED_PROBES.md) | Stream early and late reconstruction samples with exact node origins and bounded lookahead |
| [Reconstructed physical fields](docs/RECONSTRUCTED_PHYSICAL_FIELDS.md) | Complete tensors/pressure at exact probe times with retained accepted-node origins |
| [Balance quadrature](docs/BALANCE_QUADRATURE.md) | Independently refined physical-time balance integration over unchanged reconstructed trajectories |
| [Streamed residuals](docs/STREAMED_RESIDUALS.md) | Full independent defects and actual nested histories at early and late non-stage times |
| [Resources and errors](docs/RESOURCES_AND_ERRORS.md) | Complete reservations, bounded work and failure handling |
| [Contributing](CONTRIBUTING.md) | Development and evidence requirements |

The project has no dependency on Niva code, services, schemas, or credentials. An optional Niva adapter belongs in a separate downstream project and consumes the same public API as any other application.

## License

Original project material is licensed under the [Apache License, Version 2.0](LICENSE). See [NOTICE](NOTICE) and [third-party provenance](THIRD_PARTY.md). Cited papers and external repositories retain their own terms; this repository does not import or relicense them.
