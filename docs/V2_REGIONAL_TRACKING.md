# Exact-v2 regional analytical tracking

`nsbu_benchmarks::v2_experiment::reference::regional` attaches read-only to the
six accepted exact-v2 family states. It reports the existing global analytical
tracking result together with sampled Core, Annulus, InteriorOutsideNominal,
Collar and Exterior results for velocity, all nine ordered gradient entries, all
27 ordered Hessian entries and all three curl components. Pressure and its gauge
are outside this consumer.

## Reduction and coverage semantics

The consumer first completes the existing actual-versus-reference magnitude
arrays for one branch and quantity. Before that scratch is reused, it feeds the
same error and reference magnitudes to `RegionalTensorErrors`. The global result
therefore uses the same complete physical samples, component inventory, RMS
normalization and relative floor as `v2_experiment::reference`; the consumer
requires exact equality before publishing a report.

Each unshifted `[0,1)^3` sample is classified once by `regions::classify` and
contributes to exactly one of the five `SpatialRegion` entries. Counts across
those five sampled classes consequently sum to the sample-grid size. A class
with no grid points is `SampledError::NoSamples`; it is never reported as a
measured zero. These sampled classes do not establish volume coverage for
independently selected `CoveragePlan` nominal sets. Such sets can overlap, and
their completeness is a separate question.

## Separate admission and failure behavior

`RegionalTrackingPlan::new` takes an already admitted
`ReferenceTrackingPlan`. It reserves the combined six-run/tracking/regional
storage before construction, including regional report retention. In addition
to the unchanged reference, FFT and reduction ledgers, every report attempt
charges 24 complete sample-grid classifications and magnitude visits. It
conservatively charges 128 root iterations per classification, including points
whose geometry needs less work. The fixed 128 setting matches the classifier's
full bounded policy; other values are refused at admission.

Every request charges both tracking and regional worst-case work before family
identity and clock validation. Foreign, stale and exhausted requests publish no
partial report and do not advance the report schedule. A computation failure is
terminal. Successful advancement occurs only after all 24 global and five-class
findings exist. The observed spectra, state clocks, history, identity and
from-rest ownership remain unchanged.

## Bounded profile and interpretation

Run:

```sh
cargo run --release -p nsbu-benchmarks --example v2_regional_tracking
```

The profile retains the existing N=[4,8,12], steps=[64,32,16], fixed M=12 CM
family, sample grid 12, quantum 2^-20 and endpoint 128. Three reports add 124,416
classifications and magnitude visits and conservatively charge 15,925,248 root
iterations. At the endpoint, the N12/h16 CM branch's ordered-Hessian regional
sample counts and RMS errors are:

| Sampled class | Points | RMS error |
| --- | ---: | ---: |
| Core | 15 | 1.723373873675785e-4 |
| Annulus | 96 | 3.008181608432012e-4 |
| InteriorOutsideNominal | 68 | 4.940621458484716e-4 |
| Collar | 336 | 2.071234674533215e-3 |
| Exterior | 1213 | 1.713518266755864e-4 |

These nonzero values are coarse sampled errors. The analytical reference uses
binary64 jet/root evaluation, and the actual spectra use current-grid binary64
FFT arithmetic. This study provides no current-grid arithmetic bound, continuum
or regional supremum bound, monotonic convergence claim, pressure result,
arbitrary nominal-set coverage result or PDE qualification. Accepted
concentrating windows remain zero.

## Focused verification

```sh
cargo test -p nsbu-benchmarks --test v2_regional_tracking
cargo test -p nsbu-benchmarks --test v2_regional_tracking_allocation
cargo test -p nsbu-benchmarks --test v2_reference_tracking
```

The first test executes the established global consumer and regional consumer
over separately owned but identical actual-state families and checks all 72
global findings at all three clocks for exact equality. It checks complete
geometric counts, nonzero post-startup regional errors, explicit missing-class
behavior, state integrity, foreign/stale/terminal families, attempt exhaustion,
invalid root policy and cap refusal. The established tracking test retains its
independent signed full-complex Fourier oracle and high-precision analytical
fixture. The allocator executable checks allocation-free admission and report
execution within the declared joint storage bound.
