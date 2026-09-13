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

Three additional source-bound increments are recorded for review. The [review
profile](../evidence/p09/v2-review-profile/summary.json) admits nested clock
geometry and an 88-row-per-clock inventory but remains
`PartialUnpopulatedDiagnostic`: it creates no observation or measurement value.
Focused admission checks pass at 98.60% lines and 95.83% branches, with maximum
focused production CRAP 18.1181. The [physical force channel](../evidence/p09/v2-force-physical/summary.json)
is `DiagnosticOnly`; its functional, allocation and ordinary-physical checks
pass at 97.93% lines and 86.96% branches, with maximum CRAP 15. It makes no
force-sufficiency or convergence claim.

The [regional coverage increment](../evidence/p09/v2-region-coverage/summary.json)
distinguishes nominal region volume from observed sample counts and binds
actual accepted publisher state before classifying three empirical panels. Its
focused tests and controls pass, with production CRAP 23.9359 and focused plan
coverage 95.40% lines / 78.57% branches; this is focused evidence rather than
the whole-maintained release gate. Combined hosted validation for these
increments remains pending. P08/P09/P10 remain incomplete and zero
concentrating PDE windows are accepted.

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

The later P10 campaign reached the same first legal endpoint from rest on N48,
N64 and N96 with fixed M192 force sampling. Its N48-to-N64 and N64-to-N96 H1
differences were 11.3347% and 7.0493% of the respective fine-state norms, with
more than 99.997% of each squared difference in newly resolved modes. This
confirms that the old small-grid family is underresolved and supersedes it as a
resolution-planning basis. The raw states, failures and comparisons remain in
the [fixed-force spatial screen](../evidence/p10/first-endpoint-force-spatial-1b89176/README.md).

The completed P10 N192/N256/N384 matched spatial screen covers eight positive
observer clocks, 512 through 4096, with the same fixed M384 force trajectory
and piecewise schedule. At clock 512 the N256-to-N384 pair is
0.988227809391572 times the allocated H1 budget and 0.9888821775622612 times
the allocated vorticity budget; at clock 4096 those values are
0.9290197651344548 and 0.9296268180066143. At every node, the N256-to-N384 difference is smaller than the
N192-to-N256 difference. The coarse pair exceeds the allocated H1/vorticity
context budget; that is not a failure of the required spatial sequence, and the
coarse pair need not meet the fine-pair budget. This is a matched spatial screen
only; it does not establish an accepted window. See the [eight-node aggregate](../evidence/p10/snapshot-comparison-adapter/spatial-screen-8nodes/RESULT.md)
and [clock-4096 detail](../evidence/p10/snapshot-comparison-adapter/screen-clock4096/RESULT.md).

The current read-only bridge compares the actual N384 states with an
independently evaluated M768 analytical velocity, cropped to the retained band.
At clock 512 its sampled-band H1 difference is
`0.00006749991863805842`, relative `2.24157e-5`; at clock 4096 the values are
`0.0010804989124323632` and `2.1239367419e-5`. These are retained-band,
sampled-binary64 reference diagnostics, not continuum-tail, reference-precision,
trajectory-error or PDE qualification evidence. The archived [bridge summary](../evidence/p10/external-reference-bridge/results/summary.json)
and [bridge README](../evidence/p10/external-reference-bridge/README.md) retain
the source, snapshot, resource and result bindings.

The matched M384-to-M512 trajectory-force differences at clocks 512, 1024,
1536, 2048 and 3072 have H1 values of about `2.18e-5` to `2.24e-5` relative to the
M512 states, or `0.145` to `0.149` of the frozen ForceResolution pilot
allocation. Independently sampled M768 references against the M512 trajectory
give retained-band H1 differences of `3.51e-6`, `1.17e-5`, `2.33e-5` and
`2.36e-5` at clocks 512, 1024, 2048 and 3072. The M512 and HO trajectories
both reached durable clock 3072 and were then stopped because their frozen
wall-time continuation formulas could not admit the final segment. They are
useful incomplete prefixes; they do not qualify force sampling, method,
reference precision, the unresolved tail or the endpoint.

One read-only physical diagnostic evaluated the N384/M384 state at clock 512
on a 768-cubed grid. Its global RMS/peak errors were `9.74e-8`/`2.91e-6` for
velocity, `1.19e-4`/`0.00334` for the gradient, and `0.16495`/`4.1366` for the
ordered Hessian. The largest floor-normalized derivative peaks occur in the
cutoff collar and near-zero exterior, while core and annulus values are much
smaller. The [regional result](../evidence/p10/n384-regional-snapshot-diagnostic/results/clock0512.json)
uses diagnostic normalization floors rather than accepted error budgets; it
does not qualify collar volume, peak location or vorticity.

The matched N384/M512 clock-512 diagnostic completed on the identical lattice,
regions and normalization floors. Global RMS errors decreased to `7.925e-8`
(velocity), `9.800e-5` (gradient), and `0.142893` (ordered Hessian). The collar
Hessian floor-normalized peak decreased from `1.67380` to `1.16670`.
This is a measurable improvement from force refinement, with remaining errors
of the same order; the diagnostic floor is not an acceptance tolerance. See the
[matched result](../evidence/p10/n384-regional-snapshot-diagnostic/results/clock0512-m512.json)
and [execution evidence](../evidence/p10/n384-regional-snapshot-diagnostic/run-m512-clock0512.json).
The separate analytical-projection diagnostic completed without trajectory input.
Its global gradient RMS error is `9.79324e-5` and ordered-Hessian RMS error is
`0.1428795`, closely matching the actual M512-force trajectory. This strongly
supports retained N384 spatial representation as the main limitation in these
measured derivatives. These ratios are not an exact error decomposition or a
continuum bound. The next spatial diagnostic is an N512 analytical projection;
it has not run. See the [projection execution evidence](../evidence/p10/n384-regional-snapshot-diagnostic/run-analytic-projection-clock0512.json).

The first off-stage residual probe at clock 1112 shows that force-representation
differences, including the unresolved higher-frequency force, dominate that
diagnostic. Its independent M768-force residual has H1
`2147.4147`; replacing only the force term by the discrete retained M384 force
drops H1 to `0.89876`. The replacement changes the diagnostic equation and is
not a residual repair or PDE pass. The archived [localization evidence](../evidence/p10/offline-residual-probe/localization-summary.json)
also retains the failed strict arithmetic-closure checks and the separate broad
internal-consistency heuristic.

A fixed-clock force-only study at clock 1112 now separates retained-force
sampling sensitivity: the strict-N384 projected H1 difference is about `935.86`
for M384-to-M512 and `22.29` for M512-to-M768, a 42-fold contraction. The upper
N256-to-N384 band dominates both differences. This supports improving force
sampling, subject to the documented scalar/W3 execution-equivalence caveat; it
does not establish force sufficiency or a trajectory error bound. See the
[source-bound results and limitations](../evidence/p10/force-coefficient-clock1112/README.md).
A separate [exact cancellation counterexample](../evidence/p10/residual-identity-cancellation-20260913/README.md)
reproduces a squared-norm closure failure while leaving the directly assembled
residual coefficients unchanged. Neither result changes an acceptance threshold.

Analytical-reference spectrum screens provided the historical planning basis
for the completed N192, N256 and N384 spatial ladder. These screens guide grid choice; they are
binary64 sampled-reference evidence, not continuum error bounds or PDE
validation. Fixed-retained force screens separately rank M384 as the cheaper
feasibility profile and M512 as its refinement partner. The N256
[force-grid ranking](../evidence/p10/force-grid-ranking-de6dd09/README.md) and
N384 [M384/M512 comparison](../evidence/p10/force-grid-n384-baf54fa/README.md)
do not qualify either force grid without matched trajectories.

Against the already-AVX serial-component baseline, the opt-in three-lane FFT
path reduced a measured N256 integration attempt by 29.58% while preserving the
exact state hash, indicators, work and zero-allocation steady path. Its
[performance evidence](../evidence/p10/avx-w3-n256-integration-20260912/README.md)
is an execution result, not an arithmetic or PDE acceptance result. Frozen
scheduled N192/N256 owners and an experimental N384 owner retain actual-state
snapshots and explicit observer nodes without fabricating unscheduled balance
samples. They still leave the mandatory refinement and observable channels
open.

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

## Next bounded integration increment

The same-state M512 force-refinement and HO method trajectories are preserved
through durable clock 3072. Each was stopped after its frozen wall-time rule
refused continuation to 4096, so neither is an endpoint trajectory. Completed
read-only diagnostics now cover sampled references, one off-stage residual
probe and one clock-512 regional derivative screen. The next qualification
work must complete the missing refinement studies under predeclared force, time, method, residual,
pressure and regional policies. Preserve every guard, local-error, resource and
deadline refusal. Compare only identical physical and arithmetic profiles when
attributing a channel; keep sampled-reference bridge values explicitly diagnostic.

After a three-grid trajectory sequence has the required reduction and fine-pair
budget behavior, populate the full review geometry from its captured actual
states. The fine observable schedule still needs pressure, regional
derivative-sensitive quantities, qualified off-stage residual bounds, sampling, arithmetic,
reference precision, force and time channels, plus typed balances and
quadrature. Quality exceptions remain separate and attached to their exact
scope. Source-bound profile admission, the eight-node screen,
sampled-reference diagnostics and endpoint completion alone remain unqualified;
zero concentrating windows are accepted until every required channel and
lineage review passes.

See [PROTOCOL_FORMAT.md](PROTOCOL_FORMAT.md), [EXPERIMENTS.md](EXPERIMENTS.md),
and the active [P08/P09/P10 plan](../IMPLEMENTATION_PLAN.md) for the governing
requirements.
