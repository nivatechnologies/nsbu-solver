# Partial exact-v2 frozen review profile

`v2_experiment::review_profile` admits one immutable, bounded input to the generic
`FrozenProtocol`. It creates no `Observation`, supplies no numerical value and
defines no default tolerance. The caller supplies every `ObservablePolicy`; the
wrapper accepts only the exact key count and order below. Its fixed status is
`PartialUnpopulatedDiagnostic`, so construction cannot imply numerical readiness,
lineage review or a concentrating-window decision.

The mathematical identity is the raw 32-byte digest represented by
`CASE_SHA256`. The separate semantics identity is SHA-256 over a version tag, all
typed scalar descriptors and all explicit mandatory gaps. The generic protocol
then binds those identities, caller budgets, three exact time manifests and
reconstruction geometry. The profile canonical form and identity additionally
bind the source family and probe-plan identities, so equal manifests from a
different numerical family cannot collide. Its reproducible identity formula is
`SHA256(tag || family_digest || probe_digest || SHA256(generic_canonical))`;
the emitted canonical bytes are `tag || family_digest || probe_digest ||
generic_canonical`.

## Time and reconstruction profile

The nested manifests use exponent -20 and target 8192:

- coarse: `0,64,128`
- middle: `0,63,64,127,128`
- fine: `0,7,63,64,95,127,128`

`ReviewGeometry::from_manifests` accepts caller-supplied fixed-size 3/5/7
arrays and four refinements. It requires strict coarse-to-middle-to-fine nesting,
the coarse array to equal the admitted `FamilyPlan` accepted-clock manifest, and
the fine array to equal the admitted `ProbePlan` manifest. The four refinements
must be the ordered off-stage complement of those accepted clocks. The constructor
recomputes their h64, h32 and h16 nodes through the shared probe-node routine,
then records the identities derived from that immutable plan. It does not accept
caller-supplied identity words. `startup` remains the literal startup wrapper and
delegates to this validation.

All four off-stage clocks therefore retain the same admitted node geometry as
`ProbeFamily` and `ResidualFamily`: tick 7 uses `[0,64,128]`, `[0,32,64]`,
`[0,16,32]`; tick 63 uses `[0,64,128]`, `[0,32,64]`, `[32,48,64]`; tick 95
uses `[0,64,128]`, `[32,64,96]`, `[64,80,96]`; tick 127 uses `[0,64,128]`,
`[64,96,128]`, `[96,112,128]`. This admission does not assert that an owner has
published those histories. Family and probe-plan identities are part of the
complete profile canonical bytes and identity.

## Scalar inventory

The 88 stable keys begin at `0x56321001` in this order:

1. Complete-band accepted velocity differences: volume-average L2,
   nondimensional Fourier H1 including velocity and all first derivatives, curl
   L2 and divergence L2 (4).
2. Global sampled velocity, ordered gradient, ordered Hessian and vorticity,
   each with RMS error, absolute peak error and relative peak error (12).
3. Global sampled zero-mean kinematic pressure and pressure gradient with the
   same three error reductions (6).
4. Each of the five exclusive sampled classes Core, Annulus,
   InteriorOutsideNominal, Collar and Exterior, for the four field quantities
   and three error reductions above (60).
5. Complete doubled-band off-stage momentum residual with volume-average L2,
   nondimensional Fourier H1, curl L2 and divergence L2 (4). Residual units are
   acceleration units; they are not velocity errors.
6. Independently quadrature-refined energy and enstrophy balance defects (2).

Reference peaks, denominator floors, component/sample counts, layouts, peak
witness indices, `NoSamples` and `RegionEmpty` are required metadata rather than
positive-error scalar rows. A regional analytical tracking result does not
supply regional Space/Time/Method evidence: subtracting two RMS errors against a
reference is not a field difference. Unsupported channel values must remain
`Evidence::Missing`.

The semantics identity also includes nine mandatory gaps that cannot be encoded
as fabricated zero rows: analytical pressure reference, pressure gauge,
concentration peak-height discrepancy, a defined periodic peak-location error
and tie rule, nominal Core coverage, nominal Annulus coverage, collar-volume
coverage, complete global/interior qualification, and mean-momentum balance.
The two nominal coverage quantities are future `CoveragePlan` results, independent
of the five exclusive sampled classes and their point counts.

## Bounds and schedule

For 88 observables and seven fine clocks the immutable schedule has 616 rows,
with observable index varying fastest. Admission declares 3,828 policy duplicate
checks, 29,396 generic canonical bytes, 29,483 complete profile canonical bytes,
854 semantics-hash bytes and a caller
selected review-attempt allowance of at least 616. Policy storage remains
caller-owned. Canonical writing refuses a short buffer before modification.

The focused test policy uses conspicuous unit allocations solely to exercise the
API and canonical format. Those values are not benchmark tolerances or validated
scientific choices. A real growth study must freeze its own observable-specific
budget artifact before any measurements are reviewed.
