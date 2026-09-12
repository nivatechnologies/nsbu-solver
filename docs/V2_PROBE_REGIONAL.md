# Exact-v2 reconstructed-probe regional tracking

`v2_experiment::probes::regional` is a bounded, read-only consumer of completed
`ProbeFamily` and `ProbeReferenceWorkspace` publications. It measures sampled
analytical errors for velocity, ordered gradient, ordered Hessian, and vorticity
on each of the six independently evolved reconstructed branches. Every quantity
retains a global result plus the five exclusive spatial classes: core, annulus,
interior outside the nominal sets, collar, and exterior.

## Admission and ownership

`ProbeRegionalPlan::new` binds an admitted probe-reference plan, the six source
domains, the exact seven-clock manifest, a fixed 128-iteration geometry budget,
and a caller-provided joint memory cap. Its bounds include three derivative
workspaces, analytical and magnitude scratch, the retained report, all 24
regional classifications per attempt, and a small allocator allowance.

Construction allocates that admitted scratch once. `measure` performs no heap
allocation. It checks the current probe-family identity, clock, accepted-node
origins, reconstructed values and derivatives, reference sample policy, and all
six field domains before publishing. A successful call advances the schedule
only after all six branches and four quantities finish. A computation or binding
failure terminates the consumer and preserves its last complete report. An
exhausted allowance refuses the request without charging another attempt.

The consumer never assigns analytical values to an evolved state and never
changes a state, controller, accepted history, probe publication, or reference
publication. Comparison trajectories continue to evolve independently from
rest.

## What the result means

The consumer recomputes every global sampled error before regional reduction and
requires exact equality with the supplied reference publication. Both paths use
the same tracking kernel. This checks publication identity, lifecycle, resource
accounting, state integrity, and regional reduction; it is not an independent
arithmetic oracle.

The result is `DiagnosticOnly`. It does not provide continuum bounds, geometric
volume coverage, pressure or balance channels, spatial or force convergence, or
a window decision. Coordinator and JSON-export integration remain open. Accepted
concentrating PDE windows remain zero.

## Focused verification

```sh
cargo test -p nsbu-benchmarks --test v2_probe_regional --locked
cargo test -p nsbu-benchmarks --test v2_probe_regional_allocation --locked
```

The functional test exercises all seven exact clocks, all six branches, all four
quantities, full five-class sample accounting, stale and foreign publications,
attempt exhaustion, cap refusal, and state immutability. The allocation target
checks admitted construction bytes and zero steady-state allocations. The
[source-bound evidence](../evidence/p09/v2-probe-regional/README.md) records the
exact commands, resource counts, static limits, coverage input, and CRAP result.
