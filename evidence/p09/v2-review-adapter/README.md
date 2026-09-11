# Exact-v2 partial review extraction

This increment adds a fail-closed extraction from actual exact-v2 coordinator
reports. It deliberately does not construct `MeasurementReview`, apply a
numerical policy, or expose a readiness or PDE-acceptance result. Its only
status is `PartialUnqualifiedInventory`.

The extraction binds the public exact-v2 case hash, ordinary family identity,
probe identity, exact seven-clock manifest and accepted/off-stage partition.
It checks every accepted child clock and family identity, every residual
reconstruction clock and identity, physical and tracking layouts, binary64
floor words, quantity order and the source-derived finest-CM branch 2.

Each accepted clock contributes four global sampled RMS records: velocity,
ordered gradient, ordered Hessian and vorticity. These use the production
pointwise Euclidean or Frobenius magnitude followed by RMS over sample points.
The units are respectively U, U/L, U/L^2 and U/L. The analytical tracking value
comes from branch 2. Space and time are two-entry refinement sequences and
CM/HO is a method pair from the physical report. The tracking comparison's
reference is the analytical manufactured solution; the physical comparisons'
reference side is the finer/comparison numerical state already owned by those
reports.

At the accepted rest clock, measured sequence, pair and tracking values retain
actual positive-zero bit patterns. Off-stage clocks retain their actual
three-level reconstruction geometry while tracking and all eleven generic
channels are unavailable. Accepted clocks retain eight unavailable generic
channels. Missing evidence is never replaced by zero or `Floor`.

The coordinator's ten `MissingChannel` declarations are retained but are not
presented as a complete observable inventory. The result separately lists
omitted pressure/gauge, residual, regional tracking, balance/quadrature, and
peak/relative observable groups. The single floor array in `ReviewProfile`
intentionally requires the independently declared physical and tracking floor
arrays to be bitwise equal for this fixed startup profile.

One actual coordinator schedule produced 28 records from three accepted clocks
and four residual clocks. The focused harness checks bitwise value mapping,
record order, identities, layouts, floors, measured rest zeros, reconstruction
geometry, missing evidence, wrong profile/family/order, truncated input and both
resource-cap refusals. One evolved schedule is shared across those assertions.
Extraction made zero allocations or reallocations and transactionally preserved
caller output on every tested refusal.

The two caller-owned record arrays are 44,352 bytes each: one output array and
one transactional pending array. These figures are exact array sizes, not a
total stack, process, or report-copy reservation. Validation scans seven events
and publishes 28 records. An earlier `/usr/bin/time` focused run measured
1:36.51 wall seconds, 96.67 user seconds, 0.98 system seconds and 347,008 KiB
process maximum RSS, including compilation and the coordinator evolution. The
final source-matched coverage run completed successfully, but no elapsed time
was captured for it; progress estimates are not measurements.

Final focused coverage for the two production adapter files is 228/229 lines
(99.56%) and 16/20 branches (80.00%). Production plus the maintained focused
test is 447/451 lines (99.11%) and 24/32 branches (75.00%). The latter remains
below 80%; the required whole-maintained-source gate is pending combined hosted
CI. Maximum per-function CRAP is 13.125, cyclomatic complexity is 10, cognitive
complexity is 7, all-node Halstead difficulty is 37.9811, and each file is below
500 physical lines.

An initial shape used long boolean validation expressions. Its source coverage
was 206/208 lines and 45/72 branches, and maximum CRAP was 26.125. The final
cohesive typed snapshots preserve every checked field while producing the
source-bound passing CRAP result above. Both reports are retained. A broad
exploratory Clippy run without the project's `-A dead_code` policy stopped on
unrelated existing test-support dead-code warnings. The CI-matching package
all-targets/all-features Clippy command, focused Clippy, formatting, and final
warning-denied workspace Rustdoc pass.

The first design attempted to feed synthesized time sets into generic numerical
review. It was withdrawn before this commit because the coordinator owns only
one accepted `[0,64,128]` manifest and cannot satisfy that review's three strict
nested time-set and reconstruction contract. No synthetic manifest, tolerance,
reference-precision inference, qualification, P09/P10 completion, or accepted
window is present here.
