# Contributing to NSBU Solver

Start with the [implementation plan](IMPLEMENTATION_PLAN.md), [numerical design](docs/design/COMPLETE_DESIGN.md) and [scientific scope](docs/SCIENTIFIC_SCOPE.md). The independent reference and Rust numerical core are implemented; experiment verification and complete checkpoints remain in progress. Consult the root README and package evidence for demonstrated capabilities. No concentrating PDE convergence window has been accepted.

## Development checks

Follow [installation](docs/INSTALL.md), then run:

```sh
python tools/check_repository.py
python -m unittest discover -s tools/tests -v
python tools/verify_design.py --output work/design-checks.json
```

Keep generated reports, virtual environments and numerical checkpoints in ignored working directories. The design runner accepts in-checkout JSON reports only under `work/` or `evidence/` and preserves the original evidence under `docs/design/`. Commit only deliberate, compact fixtures and reviewed evidence. The Python CI workflow runs these checks and the reference tests. A separate Rust workflow checks the workspace and measures code coverage; code coverage does not establish PDE convergence. The guard tests mutate temporary copies to verify that incomplete or changed bootstrap inputs fail.

## Numerical contributions

For each change, state the problem, affected design section, resulting behavior, independent test oracle, measured errors and remaining limitations. Include negative controls when a false success is plausible. Prefer small pure functions, explicit invariants, typed failures and independent module responsibilities. Avoid shared hidden state between reference, force and integrator implementations.

Use independent fixtures rather than tests that merely restate the implementation. A production FFT cannot serve as its own oracle. A force-evaluation precision test is not a test of full integrator arithmetic. A reference-seeded local step is not an independently integrated from-rest trajectory.

User input errors return structured failures. Bounded attempts must have declared provider work and preallocated scratch. Accepted commits validate all identity/epoch tokens before changing committed state. Reference evaluators remain outside the integrator's state-update path. Carry complete error history through checkpoints and comparison lineages.

Document nontrivial formulas with equation references and derivations. Keep cyclomatic complexity low through clear mathematical decomposition; assess maintainability, numerical stability and test coverage together rather than gaming a single metric. Record complexity regressions and justify unavoidable numerical branches. Rust changes must pass format, strict Clippy, unit/integration/doc tests, package checks and the relevant independent numerical fixtures.

## Documentation requirements

Treat documentation as part of each implementation change. Explain public API
contracts, failure behavior, numerical conventions, resource ownership and
nontrivial formulas where they affect correct use. Keep architecture and module
guides aligned with the code, and test runnable examples. Installation and CLI
walkthroughs must work from a clean checkout. Clearly separate implemented
capabilities, experimental diagnostics and validated numerical results. Review
generated API documentation as well as prose; missing-documentation lint checks
do not establish that an explanation is useful.

## Changes to reviewed inputs

The imported files under `docs/design/` are frozen review inputs. A mathematical amendment needs a new revision, explicit changed assumptions, new fixtures, and a disposition of existing evidence. Do not edit both input and expected hash to hide drift. Changing a benchmark's mathematical definition requires a new case identity and invalidates incompatible restart/comparison claims.

## Independence and licensing

Public builds, tests and examples must work without Niva or any private service. Keep adapters in separate downstream repositories. Do not add private paths, credentials, integration schemas or copied proprietary code.

Contributions intentionally submitted for this project are licensed under Apache-2.0 unless explicitly stated otherwise and accepted under separate terms. Preserve third-party notices and document dependency licenses before importing source. Do not assume a cited repository without an explicit license permits source copying. No contributor license agreement is established by this bootstrap.

## Required quality review

Follow the SOLID review and exact quality thresholds in the active
[implementation plan](IMPLEMENTATION_PLAN.md). Publish actual measurements with each package; unmeasured gates remain unverified.
Coverage must reach 80% for executable lines and branches. Mutation and dead-code
findings are informational; record their scope and outcomes when those analyses run.
Never refactor frozen review artifacts or share independent numerical oracles
merely to improve a metric.
