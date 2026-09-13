# Proposed N384 M512−M384 regional force correction diagnostic

This is a read-only architecture proposal. It does not authorize loading either state payload or
running the M768 diagnostic.

The proposed diagnostic forms `delta_hat = U384M512 - U384M384` from two exact, hash-bound
clock-512 snapshots, drops both source snapshots, then samples the difference on the same unshifted
768³ lattice used by the archived M384 analytical-reference result. One reusable AVX derivative
workspace performs 39 inverse transforms: 3 velocity, 9 ordered gradient, and 27 ordered Hessian
entries. A single f64 magnitude array and one u8 region-label array are reused between quantities.
There is no analytical field evaluation, jet cache, or per-point analytical root solve.

The five labels remain exactly core, annulus, interior outside nominal, cutoff collar, and exterior.
The cheapest self-contained implementation precomputes q(z) for the 461 eligible z planes and then
fills labels with the same binary64 operations as `regions::classify`; it must reproduce the
archived count vector exactly. A later hash-bound 452,984,832-byte label sidecar could eliminate
those 461 mask-only roots, but no such sidecar is currently archived.

For each quantity and region, emit only correction RMS and sampled peak. Join those values to the
hash-bound archived M384 analytical-reference RMS/peak using reverse and forward triangle bounds:
`max(0,E384-D) <= E512 <= E384+D`. These scalar norm bounds do not reveal vector alignment,
cancellation, sign, or whether M512 is closer to the analytical reference. Carry the existing
floors only as scale metadata; an optional ratio must be named `peak_over_archived_floor`.

The exact internal reservation is 16,431,131,744 bytes (15.303 GiB), including one retained
difference state, the shared AVX catalog/workspace, one M768 f64 magnitude array, one M768 u8 label
array, 32 worker stacks, decoder overhead, allocator allowance, and bounded serialization. The
recommended external `RLIMIT_AS` is 20 GiB and the fresh `MemAvailable` gate is 33,611,000,928 bytes
(internal cap plus 16 GiB). The two sources plus newly allocated difference peak at only
4,099,145,728 bytes during decode and subtraction; both sources must be dropped before the
measurement workspace is allocated.

Runtime is estimated at 1,050–1,350 seconds, centered near 1,100 seconds, with an 1,800+60-second
launcher bound. This comes from the archived 1,909-second run minus its 853-second analytical
reference projection, plus the second decode, difference construction/hashes, and cheap label pass.
The earlier run did not record phase timestamps, so this is a review estimate rather than measured
transform-only timing.

No vorticity, acceptance, collar-volume, peak-qualification, analytical-reference, or
vector-alignment claim belongs in this output. The exact bindings, archived context values, resource
ledger, reusable call sites, missing glue, and pre-run checks are in `plan.json`.
