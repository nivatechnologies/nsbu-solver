# Next concentrating-window dependencies

No concentrating PDE window is accepted. The active [implementation plan](../IMPLEMENTATION_PLAN.md)
requires a frozen full observable/time protocol, actual-state provenance, and all
mandatory refinement evidence before P10 can record an endpoint. The current
fixed exact-v2 startup diagnostics are useful bounded measurements, not that
protocol.

## What is now available

The [diagnostic coordinator](V2_DIAGNOSTIC_COORDINATOR.md) retains accepted-state
spectral, physical, pressure, regional-reference and node-binding data separately
from off-stage residuals. The [full-event exporter](V2_DIAGNOSTIC_EXPORT.md)
preserves all seven events as raw, bounded JSON. The
[partial adapter](V2_PARTIAL_REVIEW.md) extracts only global sampled RMS velocity,
ordered gradient, ordered Hessian and vorticity; its status is permanently
`PartialUnqualifiedInventory`. These are linked to
[export evidence](../evidence/p08/v2-diagnostic-export/README.md) and
[adapter evidence](../evidence/p09/v2-review-adapter/README.md).

The [force-resolution baseline binder](V2_FORCE_BINDING.md) is now integrated
on `main` after both source-matched hosted suites passed. Numerical, allocation, CRAP and static
checks pass at source `e1dc2d3`; focused coverage is 98.51% lines and 65.22%
branches. That focused shortfall remains explicit; the required whole-maintained
80% line/branch gate passed on combined source `31e99a1` (97.98% / 87.48%). The binder remains
`DiagnosticOnly` and does not complete the partial review inventory. Its
[evidence](../evidence/p09/v2-force-binding/summary.json) retains the corrected
CRAP result (18.779) and the earlier failed/incomplete run history.

## Numerical fact to carry forward

The diagnostic-only N8/N12/N16 pilot reached tick 4096 (`1/256`) from rest, but
its full-band spatial H1 difference increased from 34.19425238632566 (N8/N12) to
39.16868231559534 (N12/N16). Full-band vorticity L2 likewise increased from
34.17856527108957 to 39.160667123299035. The N16 temporal H1 differences and
CM/HO H1 difference were much smaller, but they do not resolve the spatial
channel. See the [pilot manifest](../evidence/p09/v2-first-endpoint-diagnostic/manifest.json)
and [full-band norms](../evidence/p09/v2-first-endpoint-diagnostic/spectral-full-norms.json).

These full-band L2/H1 comparisons differ from sampled physical RMS records.
Agreement in either representation does not replace the other, and sampled RMS
is not a continuum supremum. The observed spatial trend is unresolved evidence;
it does not predict the result at another grid or force profile.

## Dependency gates

1. **Freeze a real review geometry.** `MeasurementReview` requires three strict
   nested `TestedTimes` manifests and matching `ReconstructionSamples`; the
   startup accepted list and mixed accepted/off-stage probe list are not those
   three manifests. The fine review schedule must have every required observable
   honestly measured at each fine clock. Off-stage reconstruction geometry must
   remain actual accepted-node geometry, never an analytical reset or interpolated
   accepted state.

2. **Freeze the complete observable semantics and policy inventory.** The generic
   protocol needs nonzero problem and semantics identities, policies and every
   channel rule. The inventory must cover the active-plan requirements: full-band
   derivative-sensitive comparisons, physical pressure, balances, off-stage
   residuals, interior/cutoff-collar coverage, reference and current-grid
   arithmetic. The partial adapter's four quantities and five missing observable groups,
   together with the coordinator's ten declared missing channels, remain
   incomplete evidence; these are separate inventories.

3. **Bind all eleven refinement channels on the identical profile.** Existing
   producers cover selected space/time/method, force-family, sampling,
   reconstruction, quadrature, pressure and reference work, but their scopes and
   clocks are not yet one concentrating profile. Force resolution and precision,
   current-grid arithmetic, reference precision, transfer/lineage, sampling,
   reconstruction and quadrature must remain explicit evidence. Missing evidence
   cannot be represented by zero or an unsupported floor.

4. **Bind provenance before lineage review.** Every emitted row needs the immutable
   case and protocol identities, direct-from-rest or transfer ancestry, accepted
   state/node origin, force/provider profile, clocks, resource/work records and
   artifact hashes. The JSON exporter records raw data; it is not a complete
   provenance bundle. A numerical review can reach only
   `ReadyForLineageReview`, never a PDE acceptance.

5. **Measure the unresolved spatial channel before attributing it.** A future
   admitted profile must retain its full-band L2/H1 evidence and independently
   measure force effects. It may reject the proposed endpoint; no fabricated zero,
   floor, force change, or analytical restart can repair the recorded N8/N12/N16
   trend.

## Next bounded coding increment

Implement the next frozen exact-v2 review-profile
admission layer: caller-owned three nested exact time manifests, actual
reconstruction geometry, complete observable keys/semantics identity, and a
bounded row schedule. It should admit no measurements by substitution and should
not create `Observation` rows until their required actual source exists.

Its required exit evidence is a serialized/fingerprinted profile that the generic
protocol accepts, plus focused controls showing rejection of non-nested clocks,
wrong case/semantics identity, mismatched reconstruction geometry, duplicate or
missing observable keys, insufficient capacity, and unavailable scheduled rows.
The resulting artifact remains unqualified until every channel row and the
separate provenance/lineage review are present.

See [PROTOCOL_FORMAT.md](PROTOCOL_FORMAT.md), [EXPERIMENTS.md](EXPERIMENTS.md),
and the active [P08/P09/P10 plan](../IMPLEMENTATION_PLAN.md) for the governing
requirements.
