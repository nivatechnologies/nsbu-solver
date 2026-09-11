# Alpha announcement draft

NSBU Solver alpha is available: an Apache-2.0 Rust library and CLI for forced, viscous, periodic 3D Navier–Stokes experiments.

Its first concentrating benchmark, `similarity-mms-v2`, is a manufactured case. CM and HO trajectories start from rest and evolve independently; the analytical field measures error and is never injected into the numerical state.

Implemented: Fourier pseudospectral operators, 3/2-padded products, two exponential integrators, bounded steps, resource preflight, checkpoint/resume, high-precision direct-DFT comparisons, and diagnostics.

TODO: qualify space/time/force/arithmetic refinement windows; complete pressure, balance, residual, and checkpoint-provenance checks. There are zero accepted concentrating PDE windows. The coarse N=4/M=4 run is a finite runtime diagnostic, not convergence or blow-up evidence.

OpenAI reports an analytical proof and Lean formalization for forced Navier–Stokes breakdown under Clay C/D. NSBU does not audit that proof. It studies a different manufactured force with no proved smooth extension through the target time, does not reproduce the manuscript's cascade, and does not claim the prize is settled. The separate Euler visualization project illustrates WKB/reduced dynamics whose full cascade it says is beyond direct simulation; NSBU makes no stronger proof claim.

Computational physicists and numerical-method contributors: run the examples, inspect the evidence, and help close the TODOs. Contribute: https://github.com/nivatechnologies/nsbu-solver/blob/main/CONTRIBUTING.md

Release: https://github.com/nivatechnologies/nsbu-solver/releases/tag/alpha-20260911
OpenAI article: https://openai.com/index/navier-stokes-solution/
Manuscript: https://cdn.openai.com/pdf/32d9f210-8b73-45e0-91bc-82a30aef8a9a/navier-stokes.pdf
Clay statement: https://www.claymath.org/wp-content/uploads/2022/06/navierstokes.pdf
Euler visualization: https://github.com/pmocz/euler-blowup-viz
