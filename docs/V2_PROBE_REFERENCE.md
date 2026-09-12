# Exact-v2 reconstructed analytical tracking

`v2_experiment::probes::reference` attaches a bounded read-only analytical consumer to an
admitted `ProbeFamily`. After each complete probe publication, pass both the family and its
`ProbeSample` to `ProbeReferenceWorkspace::measure`. The consumer validates the probe-family
identity, exact current clock, all six retained domains, and all accepted-node origins before
performing analytical or Fourier work.

The report compares each branch's actual reconstructed **velocity value** with the binary64
exact-v2 analytical velocity, complete ordered gradient, ordered Hessian, and physical curl.
It never interprets the reconstructed physical-time derivative as velocity. One analytical
sample grid is evaluated once per clock and reused across all six independently evolved
branches. The shared numerical kernel is also used by accepted-state reference tracking;
that API and report shape are unchanged.

Admission reserves the complete `ProbeFamily`, three derivative workspaces for its three
retained resolutions, analytical cache and magnitude scratch, one retained report, finite
attempts, a conservative 128 root iterations per sample point, all inverse transforms and
reduction visits, and publication-binding checks. Measurement performs no steady-state heap
allocation and never mutates integrated states, reconstruction scratch, clocks, histories,
or origins. A failed admitted measurement terminates the consumer and retains its previous
complete report. A failed probe producer cannot supply a complete `ProbeSample` to measure.

Focused tests exercise rest, late off-stage tick 95, and endpoint tick 128 of the bounded
startup profile on a 12-cubed physical sample lattice. The late off-stage report is compared
against an independent signed-Fourier reconstruction and direct analytical evaluator for all
six branches and all four quantities. The existing high-precision analytical tensor fixture
and accepted/regional tracking tests remain separate crosschecks.

These are global sampled binary64 errors on the current grid. They are not continuum bounds,
regional coverage, pressure or gauge measurements, force-resolution evidence, a convergence
result, or a qualified PDE window. This standalone consumer is not yet wired into the
diagnostic coordinator, CLI, or export schema.
