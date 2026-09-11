# Alpha announcement draft

NSBU Solver alpha is available: an Apache-2.0 Rust library + CLI for periodic 3D incompressible Navier–Stokes experiments.

Fourier pseudospectral discretization, CM/HO exponential time integration, exact tick clocks, bounded transactional steps, resource preflight, and checkpoint/resume. Comparison trajectories evolve independently from rest; the analytical reference never resets the numerical state.

The initial concentrating benchmark is similarity-mms-v2, a manufactured prescribed-force problem. Its purpose is to test numerical tracking of a known field and expose errors as scales shrink.

What is verified today: small-grid runtime execution, independent high-precision direct-DFT comparisons, both integrators, and restart consistency. The released source passed hosted tests and quality gates; the downloaded binary passed checksum and execution checks.

What remains: resolved concentrating space/time/force/arithmetic studies and complete scientific qualification. Zero accepted concentrating PDE windows. This alpha does not establish blow-up or reproduce the source manuscript's full construction.

For computational physicists interested in numerical verification: try the bounded examples, inspect the evidence, and report discrepancies. Linux x86_64 binary and complete source are available. Further daily alphas will ship new changes after their gates pass.

https://github.com/nivatechnologies/nsbu-solver/releases/tag/alpha-20260911
