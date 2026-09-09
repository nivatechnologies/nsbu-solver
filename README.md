# NSBU Solver

A standalone Rust project for incompressible, three-dimensional Navier–Stokes simulation and carefully qualified concentrating-flow experiments.

**Status: verified bootstrap; independent Python reference implementation in progress. The Rust runtime and simulation CLI are not implemented yet.** This checkout contains the reviewed numerical specification, exact benchmark manifest, executable mathematical checks, and an implementation plan. There are no accepted PDE convergence windows or published solver binaries.

NSBU Solver is intended to evolve all three velocity components on a periodic three-dimensional domain, with fixed positive viscosity and a prescribed force:

```text
partial_t u + (u dot grad)u = -grad p + nu Laplacian(u) + f(x,t)
div u = 0,  nu > 0
```

The first runtime will use a Fourier pseudospectral discretization, 3/2 padding for quadratic products, incompressibility projection, and exponential Runge–Kutta time integration. The numerical contract includes bounded attempts, an exact integer tick clock, transactional state commits, resource preflight, checkpoints, and independent convergence comparisons.

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

See [installation](docs/INSTALL.md) for Windows setup and [usage](docs/USAGE.md) for report interpretation and the proposed future runtime workflow. There is currently no working `cargo install` or `nsbu` simulation command.

The [independent Python reference](reference/README.md) now provides pointwise v2
scalar/jet evaluations, direct-DFT fixtures, and N=4 smooth trajectories evolved
from rest with temporal and 80/120-digit arithmetic refinements. P00B and its quality gates remain incomplete. These fixtures
do not establish a qualified concentrating PDE trajectory.

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
| [Contributing](CONTRIBUTING.md) | Development and evidence requirements |

The project has no dependency on Niva code, services, schemas, or credentials. An optional Niva adapter belongs in a separate downstream project and consumes the same public API as any other application.

## License

Original project material is licensed under the [Apache License, Version 2.0](LICENSE). See [NOTICE](NOTICE) and [third-party provenance](THIRD_PARTY.md). Cited papers and external repositories retain their own terms; this repository does not import or relicense them.
