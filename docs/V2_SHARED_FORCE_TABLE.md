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
maximum-domain lookup, binding, copy-word and strict-band transfer visits for every attempt.
The transfer bound counts destination zero-fill plus strict-band source reads and destination
writes, at most nine maximum-domain half-spectrum visits across three components. Each live
attempt is charged before binding, clock, remaining-copy or output validation. A refusal leaves caller output,
immutable table values, remaining successful-copy counts and the last complete copy record
unchanged, then terminates the owner. Replaying a terminal call adds no charge.

`SharedForceAdapterSetPlan` is the explicit bridge to the generic `SpectralRhs` and
`recorded_step` path. It admits all trajectory attempt manifests together, derives the
actual CM twelve-call or HO fifteen-call order for the full/two-half attempt, and requires
the derived clock/domain multiplicities to equal the table copy manifest. The plan counts
the table owner once, each borrowed stream and attempt manifest, and every adapter header.
Each handle then validates its exact next interval and force clock. Starting a new interval
before the prior call sequence completes, exhausting its manifest, presenting a different
limit or output, or failing a `RefCell::try_borrow_mut` terminates that handle. A borrow
conflict does not touch the table or caller output.

The `ForceLimits` exposed to `SpectralRhs` cover the adapter header and conservative
per-call table lookup, binding and strict-transfer work. The externally owned table and
borrowed manifests are covered by the joint set plan instead of being repeated in every
RHS reservation. Each velocity trajectory still owns its state, candidate, attempt
workspace, history and fresh doubled-grid `V2Observer`; those existing reservations are
preflighted separately. Observer force is evaluated afresh at 2M and is not served by this
integration table.

The focused slab fixture uses N=4/8/12, M=16 and the six branch shapes of the ordinary
family over ticks 0 through 64. Its h=16/32/64 stage streams contain 95 bounded copies at
17 unique quarter-stage clocks. Every returned coefficient word is compared with a fresh
same-M direct force evaluation on the requested retained domain. DC and every target
Nyquist plane are checked, including nonzero times. The table therefore reduces this
fixture's original-force builds from 95 to 17 without changing any returned binary64 word.
The adapter regression also advances six independently owned N=4/8/12 CM/HO trajectories
from rest through the existing recorded-step transaction. At both nonzero commits, every
state coefficient is bit-identical to a separate direct-force `Run`; outcomes, complete
histories and observer samples compare equal. Integration work ledgers intentionally
differ: the shared adapter performs no force FFT while each independently owned observer
remains fresh.

No owned `Run`, `V2Family`, archive, CLI or default mode uses this table yet. The standalone
adapter exercises the unchanged generic integrator only. Production family wiring requires
a separately reviewed ownership and slab scheduler. In particular, the current table
manifest requires a positive copy allowance for all three retained domains at every clock.
The synchronized six-stream fixture satisfies that restriction. The repository's current
`FamilyPlan` can also satisfy it because N0/N1/N2 share the finest step and its extra
temporal branches use N2. More general mixed-domain schedules with a zero-use domain at
some clocks are not admitted. There is no interpolation, retry inference, velocity-state
assignment, analytical reset, force-grid accuracy conclusion, convergence claim or
PDE-window qualification.
