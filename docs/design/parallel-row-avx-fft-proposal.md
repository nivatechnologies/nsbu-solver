# Bounded row-parallel AVX FFT proposal

Status: production design only. A standalone evidence prototype was approved and is recorded in `evidence/p10/avx-row-parallel-spike-20260912`; no solver implementation is authorized by this note.

## Decision target

The current RustFFT AVX 3D transform executes every one-dimensional row on one
thread. W3 runs three component transforms concurrently, so a triplet uses only
three transform threads and the scalar pressure transform uses one. At N384 the
12 timed RHS evaluations have occupied roughly 280--344 seconds. Separate
from-rest launches also varied materially: reported first steps span about
290--317 seconds and a second step took about 378 seconds. That spread prevents
an unpaired timing claim, but the low transform concurrency makes a bounded
row-parallel experiment credible.

The proposed first experiment keeps the current component-level W3 ownership and
adds four row participants to each component lane. Three lanes can therefore use
12 transform workers concurrently. It changes neither the default scalar FFT nor
the existing W3 constructor. The only admitted AVX lengths are 384, 576, and 768.
The N384 integration experiment would use triplet owners at RHS length 576 and
force length 384. Length 768 is retained for a separately gated scalar observer
study and is not needed to claim an RHS result.

## Existing arithmetic to preserve

`FftPlan::forward` currently:

1. validates the complete input;
2. converts each contiguous last-axis real row to complex values;
3. invokes the axis-2 RustFFT plan;
4. copies its retained half into `grid`;
5. gathers, transforms, and scatters each strided axis-0 row;
6. does the same for axis 1;
7. divides each published coefficient by the complete real sample count; and
8. validates finite Hermitian output.

Inverse execution copies the half spectrum into `grid`, transforms axes 0 and 1,
reconstructs each last-axis conjugate row in the same index order, transforms it,
and publishes its real part. The axis order remains 2, 0, 1 forward and 0, 1, 2
inverse. RustFFT 6.4.1 declares `Fft` as `Sync + Send`; immutable plan `Arc`s may
therefore be shared while every mutable transform buffer and scratch region has
one owner.

The row executor must call `process_with_scratch` once per row, with the same
length and direction as the scalar implementation. It must not use a different
multi-row RustFFT entry shape in the first experiment. A worker processes its
assigned rows in ascending global row order. Concurrency changes the order among
independent rows only, so every coefficient follows the same floating-point
instruction path. Scaling, Hermitian validation, and final publication remain
serial and in their current order. Bitwise equality is therefore an admission
condition, not a tolerance target.

The current AVX path copies the in-place RustFFT result from `input` to `output`
before scattering it. The proposed packed row itself is the in-place input and
the scatter reads it directly. This removes a bit copy and does not change any
arithmetic.

## Safe owned-block protocol

Each existing W3 component worker remains the sole owner of its component grid,
physical lane, and spectral lane. It becomes a coordinator and one of four row
participants. Three persistent helper threads are created under that component
worker. No helper receives a grid pointer or a borrowed slice that outlives a
call.

Each row participant has two owned `Vec<Complex64>` slots of exactly
`BLOCK_ROWS * length` elements. A fixed `BLOCK_ROWS = 256` bounds memory while
amortizing dispatch. Slot state is `Free`, `Ready(Job)`, `Running(Job)`, or
`Complete(Job, Result)`. The coordinator fills a free slot while another slot is
being transformed. A helper moves a ready vector out of shared state, releases
the mutex, transforms its rows using thread-local scratch, then moves the vector
back as complete. At every transition exactly one thread owns the vector.

Jobs use monotonically increasing sequence numbers and a fixed round-robin
worker assignment. The coordinator drains and scatters completions in sequence
number order. Dynamic work stealing is excluded. There is one complete barrier
between axes because the next axis reads every result of the previous axis.

For a cubic length `L`, component `c`, and retained last-axis length
`H = L / 2 + 1`, the deterministic row identities are:

| Axis | Transform count per component | Grid address for element `j` |
|---|---:|---|
| 2 | `L * L` | contiguous physical row; retained scatter to `row * H + j` |
| 0 | `L * H` | `y * H + k + j * L * H` |
| 1 | `L * H` | `x * L * H + k + j * H` |

The coordinator alone performs these gathers and scatters. Every transform row
within an axis covers a disjoint set of grid coefficients. This avoids `unsafe`,
aliased `&mut` slices, a shared grid mutex in the compute loop, and a transpose.
Packing and scattering preserve the scalar loop's `j = 0..L` order.

With component width `C`, the number of dispatched blocks is
`C * (ceil(L*L/256) + 2*ceil(L*H/256))` per 3D transform batch because each component has a separate owner. For W3 this is
3,468 blocks at L384 and 7,794 at L576. A measurement must report packing,
RustFFT work, completion waiting, and scattering separately; a transform total
without these costs is not admissible evidence.

## Ownership, construction, and failure

The new type should be private to `spectral::w3`, for example
`RowParallelW3Owner`. It consumes the same three validated scalar lanes as W3
and exposes the same narrow operations: `prepare_inverse`, `inverse3`,
`forward3`, `forward_one`, `with_spectrum`, `identity`, and `is_terminated`.
Existing serial and W3 owners, constructors, traits, and defaults remain intact.
A distinct opt-in constructor selects row execution and records length, backend,
component width, four participants per lane, block size 256, two slots, direction
mode, and added reservation in its execution identity.

RHS and force continue to own separate pools. There is no global executor or
shared mutable handle, and the force sampling pool does not borrow the FFT pool.
Sampling and transforms are sequential phases, so the 32 sampling workers and 12
active transform participants do not multiply into nested active work.

Construction validates the consumed plans, backend, layout, workspace shape,
capacities, closed length, complete reservation, and CPU policy before allocating
or spawning. Helper slots and scratch are allocated before thread startup. A
partial startup failure signals and joins every started helper before returning.
No allocation is permitted after construction.

Every external input is validated before dispatch. RustFFT calls execute behind
the same panic boundary as the current W3 workers. On the first numerical,
protocol, lock, or caught transform panic failure, the coordinator stops filling
new blocks, drains every ready/running block, discards all completed private
results, and permanently terminates the component owner. W3 drains all three
component owners before returning. Private grid, slot, and FFT scratch contents
are unspecified after failure. RHS output, force cache slots, candidate state,
and committed state publish only after all components succeed. There are no
caller callbacks under row-worker locks.

## Exact declared reservation

The first experiment fixes:

- `C = 3` component lanes;
- `K = 4` row participants per lane, including the existing component worker;
- `B = 256` rows per slot;
- two slots per participant;
- 2 MiB stack per helper;
- 64 KiB thread-system allowance per helper;
- 2,048 bytes of declared row-worker metadata;
- the existing W3 worker metadata allowance of 944 bytes; and
- 64 bytes per new allocation.

The first participant in each lane reuses that lane's existing `4L` AVX scratch.
The other `K-1` participants receive private `4L` complex scratch. The exact
declared addition over the current W3 owner is:

```text
slots       = 2 * C * K * B * L * 16
scratch     = C * (K - 1) * 4 * L * 16
helpers     = C * (K - 1) * (2 MiB + 64 KiB + 2,048)
coordinator = C * (2,048 - 944)
allocations = 3 * C * K * 64
delta       = slots + scratch + helpers + coordinator + allocations
```

`3*C*K` allocations are two slot vectors per participant, one added scratch
vector per helper, and one helper-vector allocation per component. Immutable
plan `Arc` clones allocate nothing. An implementation must statically assert
that worker metadata fits 2,048 bytes and must use checked arithmetic for every
term.

| Cubic length | W3 row addition, 12 active participants |
|---:|---:|
| 384 | 57,458,160 B before concrete shared-owner audit |
| 576 | 76,443,120 B before concrete shared-owner audit |
| 768 | 95,428,080 B |

The 133,901,280 B sum for N384 RHS576 plus force384 is a design candidate pending a concrete audit of separately allocated shared owner state, `Arc` control blocks, vectors, and thread startup allocations. It must not be used as an admission reservation until that audit is complete.

A future scalar length-768 owner would use 12 participants, with the caller as
coordinator, 11 helpers, and the existing scalar scratch for its first
participant. Its exact declared addition is:

```text
2*K*B*L*16 + (K-1)*4*L*16
+ (K-1)*(2 MiB + 64 KiB + 2,048) + 2,048 + 3*K*64
```

At K12/L768 this is 99,854,592 B. If all three N384 owners were admitted, their
combined addition would be 233,755,872 B and the total would be 186,016,917,480
B, leaving 20,141,512,728 B under a 192 GiB cap. The observer owner remains out
of the first RHS experiment.

## Worker placement and first touch

The measured development host has one NUMA node, 64 physical cores, 128 hardware
threads, and 16 L3 instances. Twelve active transform participants avoid SMT and
fit below both physical-core and L3-slice counts. The harness should run inside a
fixed 12-core CPU set and record the CPU set and NUMA topology. The solver should
not add a global affinity policy.

Current W3 construction allocates and zero-fills its large lane buffers on the
constructor thread before starting the component workers. On a multi-node host,
an unbound worker may then consume pages first-touched on another node. A smaller
alternative experiment would bind each component worker first and allocate and
zero its lane inside worker startup, while retaining pre-spawn reservation and
partial-startup drainage. This may improve locality with much less code than row
parallelism. It cannot be claimed on the present single-node host as a remote-page
fix, and the observed 290--317 second first-step and 378 second second-step spread
is larger than a plausible locality conclusion. Test worker-local first touch
first on any multi-node target; keep it only if two interleaved pairs improve at
least 10% with unchanged CPU capacity and exact outputs.

## Interfaces and experiment sequence

The narrow code path would touch only:

1. `spectral/fft`: an internal AVX-axis kernel accessor and row-worker module;
2. `spectral/w3`: the distinct row-parallel owner and its closed admission;
3. `spectral/rotational`: one opt-in owner variant and constructor;
4. `integrators/rhs`: one opt-in reservation/constructor/identity seam; and
5. the parallel reduced-force experiment wrapper with its own row pool.

The existing harness-local timed RHS delegate can wrap `RightHandSide` unchanged
and measure all 12 evaluations. No coefficient cache, profiler framework,
transpose, scheduler API, active binary, or default changes are needed.

Before an N384 step, a fixture must prove bitwise forward/inverse equality,
direct-reference agreement, exact cap refusal, zero steady allocations, every
helper's numerical/panic drainage, permanent termination, and no external
publication. Then a transform harness measures current W3 against row W3 for
forward and inverse triplets at 384 and 576 and scalar forward/inverse at 768.

Proceed to N384 only if each relevant 384/576 triplet composite, including pack,
dispatch, waits, and scatter, is at least 1.8 times faster than current W3, no
direction regresses, and non-RustFFT overhead is below 35% of row-owner wall
time. Length 768 remains an unqualified guard study.

The actual integration gate uses two interleaved current-W3/row-W3 pairs from the
same N384 rest state and fixed CPU set. It requires:

- exact equality of state hash, indicators, work, cache hit/miss, and balance;
- zero steady allocations and exact execution identities/reservations;
- at least 15% integration reduction in each pair;
- at least 25% reduction in the median paired integration time; and
- at least 25% reduction in the harness delegate's aggregate 12-RHS time.

Observer time is reported but excluded from the first RHS gate. The existing
280--344 second RHS range is context only. Cross-launch 290/317/378 second step
values cannot satisfy the paired gate. If either the composite transform gate or
the N384 paired gate fails, preserve the negative result and stop before any
default or active-experiment integration.
