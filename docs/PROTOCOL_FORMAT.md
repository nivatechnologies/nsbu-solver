# Frozen numerical protocol identity

`nsbu_solver::verification::protocol::FrozenProtocol` binds admitted numerical
rules, exact tested-time manifests and reconstruction geometry to one SHA-256
identifier. It can write the complete canonical artifact into a caller-owned
buffer. One encoder feeds hashing and serialization.

This is a generic identity component. The benchmark must still verify the
mathematical problem, complete observable inventory, units, masks and interpretation
represented by the supplied identifiers. A matching fingerprint does not
authenticate those identifiers, measurements or from-rest trajectory origin.
The component cannot issue an accepted PDE window.

## Construct and use a protocol

Prepare `ProtocolInputs` with nonzero mathematical-problem and observable-semantics
identifiers, admitted `Policies`, three nested `TestedTimes` manifests and admitted
`ReconstructionSamples`. The semantics definition must cover diagnostic units,
region definitions, relative-error floor rules, peak/location conventions and the
complete mandatory observable inventory. Integer keys alone cannot establish
those semantics.

`reservation(inputs)` computes the exact byte traversal using checked arithmetic.
`FrozenProtocol::new(inputs, maximum_encoded_bytes)` refuses an inadequate cap
before hashing. It also requires an admissible numerical-review schedule: strict
time refinements and reconstruction on the same fine manifest. The protocol
borrows its arrays, preventing mutation while it remains live.

This adapter accepts already validated caller inputs:

```rust
use nsbu_solver::verification::{
    protocol::{FrozenProtocol, ProtocolInputs}, VerificationError,
};

fn encode_protocol(
    inputs: ProtocolInputs<'_>, output: &mut [u8], maximum_bytes: usize,
) -> Result<([u8; 32], usize), VerificationError> {
    let frozen = FrozenProtocol::new(inputs, maximum_bytes)?;
    let written = frozen.write_canonical(output)?;
    Ok((frozen.identity(), written))
}
```

`write_canonical` rejects a short buffer before modification and leaves trailing
caller bytes unchanged. Construction, refusal, hashing, serialization and review
creation allocate nothing after the caller constructs its manifests. The byte cap
bounds canonical traversal work; caller-owned manifest storage and upstream
admission have separate budgets. It is not a wall-clock bound.

`frozen.review(maximum_attempts)` creates a bounded `MeasurementReview` using
exactly the frozen settings. An insufficient attempt cap is refused. A fresh
review remains `Incomplete`; numerical success reaches only
`ReadyForLineageReview`. Attempt caps are execution limits recorded separately
from scientific protocol identity.

## Canonical version-one bytes

Integers and binary64 words are little-endian. Counts are u128 and observable keys
are u32. There is no platform-sized integer encoding, padding, string terminator
or appended digest. Identity is SHA-256 of the entire following sequence.

| Order | Content |
| --- | --- |
| 1 | Exactly 16 ASCII bytes: `NSBUPROTOCOL0001` |
| 2 | Mathematical problem identifier: 32 bytes |
| 3 | Observable-semantics identifier: 32 bytes |
| 4 | Policy count: u128 |
| 5 | Every policy in declared review order |
| 6 | Coarse, middle and fine time manifests, each prefixed by its u128 count |
| 7 | Reconstruction-refinement count: u128 |
| 8 | Every refinement in declared probe order |

Each policy encodes its u32 key, total-budget binary64 word and conservatively
accumulated allocation binary64 word. All eleven channels follow in public
`CHANNELS` order. Each contributes binary64 words for budget, maximum reduction
ratio and **effective absolute subordinate-floor budget**, followed by one
requirement byte: zero for `Sensitivity`, one for `Refinement`. Constructor
inputs producing identical effective rules have identical encodings. Preserve an
original input-file artifact separately when its exact syntax also matters.

Each clock encodes its signed i32 exponent, then u128 target, elapsed and remaining
counts: 52 bytes. Each reconstruction refinement encodes three levels in
coarse-to-fine order; each level contains three node clocks followed by its probe
clock. Repeated clocks are included. Clock representation, time-set membership
and every reconstruction interval therefore participate in identity.

For `O` policies, time-set lengths `T0,T1,T2`, and `P` reconstruction refinements:

```text
encoded bytes = 160 + 295*O + 52*(T0+T1+T2) + 624*P
```

The independent format fixture uses one observable (key 41), problem bytes all 1
and semantics bytes all 2. Total budget is 1; every channel has budget 0.01,
reduction ratio 0.5, constructor floor fraction 0.1 and requirement `Refinement`.
Clock exponent is −16, target 512. Tested tick sets are `[0,128]`, `[0,64,128]`
and `[0,64,127,128]`. Reconstruction nodes are `[0,64,128]`, `[64,96,128]` and
`[96,112,128]`, all probing tick 127. Its 1,547 bytes hash to
`72c400332fd6633c307faa972a614220e33a653241bfd51f6288348511cf7354`.
Python integer/struct encoding and exact rational checks of upward-rounded
allocation arithmetic independently produced this fixture.

## Verification and integration scope

```sh
cargo test -p nsbu-solver --test protocol --test protocol_allocation
```

Tests cover every channel's budget, reduction and floor, requirement changes,
problem/semantics identity, key order/count, total tolerances, all time levels,
probe changes, history refinement, missing identities, short buffers and bounded
review creation. The isolated allocator executable checks successful and refused
operations without test-harness allocation interference.

The format carries settings and geometry, without integrated Fourier coefficients
or reference/force measurements. There is no decoder that promotes external bytes
into trusted benchmark semantics. Complete inventory binding, actual-state and
artifact provenance, and full experiment/window qualification remain required by
the [active implementation plan](../IMPLEMENTATION_PLAN.md).
