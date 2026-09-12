# Design-only W3 execution-owner proposal

## Scope and evidence boundary

This is a proposal for root review, not production code or qualification. It
does not alter `Run`, a default API, the FFT arithmetic, the active N192
endpoint, or its artifacts. The N192 scheduled-cost decision remains negative:
21.99% swap-published and 20.54% copy-inclusive reductions both miss 25%.

The N256/M384 model below is narrow. It combines this spike's N384 medians with
the one accepted from-rest h32 step and explicit startup observer recorded by
`codex/p10-fft-batch-20260912@2593553`. That startup observation is timing
evidence rather than a scheduled node. It establishes neither a later-state
profile nor arithmetic, force-grid, temporal, quadrature, PDE-window, or
trajectory qualification.

## Owner choice and interface

Use one opt-in `W3FftExecution` owned by the experimental endpoint execution.
Pass a mutable, non-storable batch handle into `SpectralRhs` and, on cache
misses, through it into the reduced-force transform. Do not put separate W3 FFT
pools in the RHS and force provider. At N256/M384 the single owner can reuse the
same N384 lanes for both call sites; independent owners would add duplicate
workers, workspaces, and staging fields. At N384 with two transform layouts a
single owner still gives one request stream and one failure boundary, while the
layout-specific workspaces and lanes remain distinct.

The interface should admit a fixed identity and expose only complete component
triples, conceptually:

```text
forward3(layout, &mut [Vec<f64>; 3], &mut [Vec<Complex64>; 3]) -> Result<Receipt>
inverse3(layout, &mut [Vec<Complex64>; 3], &mut [Vec<f64>; 3]) -> Result<Receipt>
```

Each worker owns its `FftWorkspace`, spare input vector, and output staging
vector. Submission swaps the caller's input Vec into the lane, so the worker
owns every cross-thread buffer. Drainage swaps each input back. Only complete
success swaps staged output into the caller's destination, in component order
0, 1, 2; failure returns inputs while leaving prior destinations untouched. A
receipt identifies the execution generation, layout, direction, and three
logical destinations. The owner retains no caller borrow across a worker
operation and performs no steady-state allocation.

One RHS call submits these batches without changing any scalar transform's
operation order:

1. inverse velocity components on the rotational padded layout;
2. inverse vorticity components on that layout;
3. after the serial pointwise cross product, forward its three components;
4. leave the scalar kinetic-energy/pressure forward transform serial and in its
   current position.

Each reduced-force cache miss submits one forward triplet on the configured
force sampling layout after the existing 32-worker point sampling completes.
A cache hit submits no FFT request. The h32 Cox-Matthews attempt has 12 RHS
calls with seven hits and five misses, so the exact batch inventory is 24 main
inverse triplets, 12 main forward triplets, and five cached-force forward
triplets. The 12 scalar pressure forwards remain unchanged. Thus N256/M384
models 24 inverse and 17 forward N384 batches, or 123 batched scalar transforms,
plus 12 unchanged scalar transforms.

## N256/M384 scheduled model

The observed from-rest step took 184.520150132 s. The explicit current observer
took 158.601201704 s and is conservatively amortized at eight positive nodes per
128 attempts, or 1/16. The scheduled baseline is therefore 194.4327252385 s.
The N384 component accounting is:

| term | serial (s) | W3 swap (s) | W3 plus copies (s) |
|---|---:|---:|---:|
| 24 inverse triplets | 51.961128672 | 21.319451088 | 23.276381160 |
| 17 forward triplets, including five force misses | 36.549244367 | 14.448731222 | 15.895079152 |
| batched subtotal | 88.510373039 | 35.768182310 | 39.171460312 |

Replacing only that subtotal gives integration costs of 131.777959403 s and
135.181237405 s. Leaving the observer unchanged gives scheduled costs of
141.6905345095 s (27.1262% lower) and 145.0938125115 s (25.3758% lower).
The copy-inclusive result clears 25% by only 0.3758 percentage points. Dispatch
is already inside the W3 timings; no observer transforms are accelerated.

This is a narrow modeled pass and a reason to request a bounded implementation,
not an endpoint admission. Measurement noise, a later-state RHS mix, nested
thread contention with the 32 force workers, integration copies, or interface
cost can remove the margin. The implementation gate must use a source-matched
whole h32 step and scheduled observer cost and must still clear 25% with all
copies and dispatch charged.

## Reservation

Keep admission conservative for the first implementation: retain the existing
serial owners and add the complete W3 reservation before constructing a lane or
thread. The exact N384 bidirectional addition is 2,733,911,936 B. Separate N384
RHS and forward-only force pools would add 2,733,911,936 + 1,827,942,144 =
4,561,854,080 B. A single same-layout owner therefore avoids 1,827,942,144 B
of duplicate admitted addition. Exact-cap construction must succeed and
one-byte-short construction must refuse before allocation.

For a future retained N384, integration M384 design, the main rotational layout
is N576 and the force layout is N384. The exact conservative addition for one
owner is 9,200,779,136 + 1,827,942,144 - 6,490,960 = 11,022,230,320 B: the N576
bidirectional lanes, N384 forward lanes, and one rather than two worker/thread
allowances. The N384/M512 case needs an exact N512 workspace/lane derivation
before admission; this spike did not measure or reserve N512. An N576 capacity
must not be asserted as an exact N512 reservation.

## N384 h32 future guard study

There is no measured retained-N384 h32 integration or observer baseline, and no
N512 W3 transform timing. The following is an unqualified component guard only.
It uses 12 main RHS inventories at N576, the fixed-retained-N256 clock-4096 force
measurements of 9.290477717 s for M384 and 24.273537816 s for M512, and five
force misses. Those force measurements differ in retained layout, clock, and
execution context from the proposed N384 h32 run.

For integration M384, the modeled serial component is 299.750124169 s. The
swap-published component is 124.160611300 s (58.58% lower); copy-inclusive is
134.749082886 s (55.05% lower). Here both main and force transforms use measured
same-length N576 and N384 batch pairs.

For integration M512, bracket the unmeasured force-transform saving by the
measured same-length N384 and N576 pair savings. The serial component is
374.665424664 s. Swap-published cost ranges from 183.038615430 to
199.075911795 s (46.87--51.15% lower), and copy-inclusive cost ranges from
194.640065866 to 209.664383381 s (44.04--48.05% lower). This interpolation
bracket is not an N512 timing or performance bound. Neither M384 nor M512 result
can be promoted to a whole-step percentage until a source-matched retained-N384
h32 profile measures integration, force misses, observer costs, and contention.

## Failure, identity, and validation contract

Submit all lanes, record partial submission, and drain every submitted lane even
after the first error or caught panic. Publish none unless all three lanes
succeed. A failed generation leaves every caller destination and cache slot at
its prior generation, marks the W3 owner terminated, and makes later requests
refuse. Cache-slot epoch publication follows successful force-triplet
publication; it cannot turn a failed miss into a hit.

Bind an experimental execution identity to source commit, case hash, explicit
profile (`n256-m384-w3` initially), RustFFT 6.4.1 AVX/AVX2/FMA backend, width 3,
layout/direction inventory, publication mode, owner mode, and exact reservation.
Preflight and artifacts must reject any mismatch. Select it only through a new
experimental harness feature or binary; the default N192 identity and the
currently running endpoint remain unchanged and cannot resume from W3 output.

Focused implementation tests should cover exact-cap and one-byte-short
admission; serial/W3 word equality for both inverse triplets, the nonlinear
forward triplet, and a force-cache miss; no dispatch on a cache hit; the exact
7-hit/5-miss h32 schedule; repeated whole-attempt state hashes; direct-reference
small transforms; zero steady allocations; every single-lane error and panic;
partial-submit drainage; no partial RHS, force cache, attempt, or endpoint
publication; terminated-owner refusal; and identity/default-profile mismatch
refusals. Run format, all-target clippy, focused tests, rust-code-analysis, CRAP,
then a source-matched whole-step comparison with copies and dispatch included.

A bounded N256 implementation and focused controls should take 4--6 hours. Two
interleaved whole-step confirmation pairs fit in another 2--3 hours at the
recorded baseline, so root can make the integration decision within one working
day. Retained N384 needs a separate N512 reservation/timing study when M512 is
considered, plus a measured h32/observer profile; allow an additional working
day before any whole-step gate is claimed.
