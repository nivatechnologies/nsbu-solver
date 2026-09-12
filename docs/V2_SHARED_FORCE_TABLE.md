# Exact-v2 shared original-force slab table

`v2_experiment::shared_force` is a standalone integration-scale owner for one explicit
clock slab. It evaluates the original exact-v2 force once at each admitted clock on the
largest retained domain, then freezes those coefficient arrays. Sequential independent
consumers can request a smaller admitted strict band through `SharedForceTable::copy`.
Only copy allowances, failure state, work charges and the last complete copy record mutate;
the force coefficients do not.

`SharedForceTablePlan` binds the `similarity-mms-v2` case hash, force sample grid M,
worker count, three strictly nested retained domains, their physical lengths and viscosity,
the complete full-`TickClock` manifest, positive per-clock/per-domain copy counts and a
finite total attempt allowance. Its identity includes all of those fields. A binding from a
plan with a different M, worker selection, domain, clock manifest or attempt cap is foreign.
The exact-v2 provider additionally restricts the current implementation to unit lengths and
unit viscosity.

Admission counts the borrowed manifest separately. Persistent owned storage includes every
maximum-domain coefficient entry and metadata. Construction peak adds exactly one original
force provider; runtime peak instead adds the largest caller-owned output. The joint cap
covers the larger peak. Work reserves one provider evaluation per clock and conservative
maximum-domain lookup/copy work for every attempt. Each live attempt is charged before
binding, clock, remaining-copy or output validation. A refusal leaves caller output,
immutable table values, remaining successful-copy counts and the last complete copy record
unchanged, then terminates the owner. Replaying a terminal call adds no charge.

The focused slab fixture uses N=4/8/12, M=16 and the six branch shapes of the ordinary
family over ticks 0 through 64. Its h=16/32/64 stage streams contain 95 bounded copies at
17 unique quarter-stage clocks. Every returned coefficient word is compared with a fresh
same-M direct force evaluation on the requested retained domain. DC and every target
Nyquist plane are checked, including nonzero times. The table therefore reduces this
fixture's original-force builds from 95 to 17 without changing any returned binary64 word.
This is a force-owner test, not a trajectory run.

No `Run`, `V2Family`, integrator, archive, CLI or default mode uses this table yet. Production
family wiring requires a separately reviewed ownership and slab scheduler. The doubled
observer force uses 2M and doubled retained domains and must receive a separate table plan,
identity, provider and resource reservation. There is no interpolation, retry inference,
velocity-state assignment, analytical reset, force-grid accuracy conclusion, convergence
claim or PDE-window qualification.
