# Next spatial and force trajectory profile

This is a source-linked planning note. It prepares no trajectory, changes no
numerical requirement, and authorizes no execution. The completed N384/M512 r6
trajectory remains the current endpoint anchor.

## Existing and missing anchors

The completed matched spatial screen contains N192/M384, N256/M384, and
N384/M384 at the same eight positive clocks and under the same piecewise time
profile. The independently evolved r6 trajectory adds N384/M512 under that same
time, method, tolerance, and advective policy. It is the only committed M512
trajectory anchor. No committed N256/M512, N512/M512, N384/M768, or N512/M768
trajectory is currently recorded; the N512/M768 attempt is timing evidence with
no committed or published candidate.

The sampled-force provider requires every force-sample dimension to be at least
the retained dimension in
[`provider/reduced.rs`](../../../crates/nsbu-benchmarks/src/provider/reduced.rs).
Consequently N512/M384 is refused, while N512/M512 and N512/M768 are legal
provider geometries. A future N512/M768 state cannot be compared directly with
N384/M512 as spatial refinement because both retained and force grids change.

The smallest useful next trajectory is therefore N512/M512 with the r6
piecewise schedule and unchanged numerical policy. It creates an isolated
N384-to-N512 spatial pair at fixed M512. A later N256/M512 trajectory completes
the natural N256/N384/N512 spatial family. Neither run alone accepts a window;
the full observable schedule still needs pressure, regional derivative
quantities, off-stage residuals, sampling, arithmetic, reference precision,
time, method, balances, quadrature, and lineage evidence.

## Force-resolution obligation

The frozen revision-0.7 design requires retained force coefficients from at
least two increasing evaluation grids together with independent off-grid probes
or analytic coefficient/derivative bounds. It also requires separate force
precision evidence. A runtime-grid tail or force-only coefficient ranking is a
diagnostic and cannot qualify the channel. See
[`COMPLETE_DESIGN.md` section 4.3](../../../docs/design/COMPLETE_DESIGN.md#43-force-sampling-is-a-separate-error-channel)
and its [minimum refinement family](../../../docs/design/COMPLETE_DESIGN.md#63-minimum-refinement-family).

The generic verifier reflects that contract:
[`ForceResolution`](../../../crates/nsbu-solver/src/verification/budget.rs) may
use a predeclared sensitivity rule, and
[`Evidence::Pair`](../../../crates/nsbu-solver/src/verification/refinement.rs)
passes only when the independently measured change is strictly below its
allocated budget. Identity, actual-state provenance, alias control, and
precision evidence remain separate obligations.

The three-trajectory
[`ForceFamilyPlan`](../../../crates/nsbu-benchmarks/src/v2_force_experiment/plan.rs)
uses strictly increasing, divisibly nested force grids. That is the contract of
this bounded producer, not a replacement for the revision-0.7 requirement. A
matched future N512/M512 and N512/M768 pair can supply the two-grid trajectory
sensitivity input only if every frozen observable and time is populated and the
independent alias and precision obligations also pass. No such qualification is
claimed or prepared here.
