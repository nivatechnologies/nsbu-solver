# Partial exact-v2 measurement extraction

`v2_experiment::review_adapter` extracts four global sampled RMS observables
from a complete seven-event diagnostic report set: velocity, ordered gradient,
ordered Hessian and vorticity. It binds the case, family, probe manifest,
branch 2 (finest CM), sample layouts and exact relative-floor words.

The output is always `PartialUnqualifiedInventory`. This API does not construct
`MeasurementReview`, apply tolerances, or decide whether a PDE window passes.
The startup coordinator does not own the three nested observation schedules
required by the full verifier.

## Library use

After evolving a `DiagnosticDriver` through its complete admitted schedule,
extract the retained reports with caller-owned output storage:

```rust
use nsbu_benchmarks::v2_experiment::review_adapter::{
    extract, AdapterBounds, ReviewProfile, REVIEW_RECORDS,
};

let settings = driver.plan().diagnostic_settings();
let profile = ReviewProfile {
    physical_samples: settings.physical_samples,
    tracking_samples: settings.reference_samples,
    relative_floors: settings.physical_floors,
};
let bounds = AdapterBounds::fixed();
let mut records = [None; REVIEW_RECORDS];
let result = extract(
    driver.plan(), profile, driver.reports(), REVIEW_RECORDS,
    bounds.transactional_bytes, &mut records,
)?;
```

This is a fragment for a caller that already owns the driver and handles errors;
it is not a standalone executable. The fixed profile requires physical and
reference-tracking floors to be bitwise equal. Its output and internal pending
arrays each occupy 44,352 bytes on the measured target. These array sizes are
not a bound on total stack or process memory; driver and report storage remain
separate reservations. Extraction itself performs no heap allocation.

## Interpretation

Accepted clocks provide space and time sequences, a CM/HO pair, and analytical
tracking from the independently evolved finest CM branch. At rest, a measured
zero remains a measurement. Off-stage clocks populate space, time and method
from actual reconstructed-value physical comparisons and are marked
`OffstagePhysicalMeasured`. They also retain the sampled analytical RMS from
the actual reconstructed-reference producer at the same manifest clock, using
its finest CM branch. This tracking value does not replace a physical pairwise
difference; every other channel remains `Missing`. Their actual reconstruction
geometry is retained separately. Accepted records continue to use ordinary
integrated-state physical findings, so the two sources remain explicit.

The result preserves explicit missing observable groups and channels. Pressure,
regional, residual, balance and peak observables are not silently represented
by these four RMS quantities. RMS uses pointwise Euclidean/Frobenius magnitudes
and divides by sample count, never by component count.

See the [diagnostic coordinator](V2_DIAGNOSTIC_COORDINATOR.md),
[verification protocol](PROTOCOL_FORMAT.md), and
[source-bound tests and quality evidence](../evidence/p09/v2-review-adapter/README.md).
