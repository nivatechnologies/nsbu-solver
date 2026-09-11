# Exact-v2 accepted-node binding

`nsbu_benchmarks::v2_experiment::binding` links two independently evolved
owners without treating reconstruction output as numerical state. The ordinary
`V2Family` remains the source of endpoint diagnostics. `ProbeFamily` remains a
separate population of six reconstruction-capable runs whose lookahead may put
current states after the ordinary family's accepted clock.

## Retained-node provenance

The reconstruction observer exposes only a crate-private borrowed view of an
exact node still present in its three-node accepted ring. The view contains the
stored clock, epoch, accepted-step count and three velocity coefficient slices.
These fields are captured with the accepted state; the binder does not derive or
invent them from the requested clock. The view provides no mutable storage,
derivative scratch, snapshot ownership or interpolation method.

`ProbeFamily` binds that view to its fixed branch slot, retained-grid domain,
`InternalFromRest` origin, underlying ordinary-family identity and distinct
probe-manifest identity. The binder separately requires the supplied ordinary
and probe families to match the two admitted plans. A located node whose branch,
identity, origin, domain, clock, epoch or accepted-step count contradicts the
ordinary state is `InvalidAcceptedNode`; no partial report is published. A stale
ordinary clock, foreign family or terminal probe owner remains a family refusal.

If the exact requested clock is absent from one branch's retained ring, the
complete report records `MissingRetainedNode` for that branch. Absence can mean
the node has not yet been generated or was evicted by lookahead. The binder never
substitutes the probe owner's later current state or an interpolated field.

For a valid retained node, every complex coefficient word is compared with the
corresponding ordinary accepted state. `coefficients_equal` preserves both
positive and negative evidence: false is an observed mismatch, not an error
bound, tolerance result or acceptance decision. Signed-zero and one-ULP negative
controls check the word comparison independently.

## Admission and requests

`NodeBindingPlan::new` verifies that `ProbePlan` wraps the same immutable family
identity and uses the ordinary family's exact accepted-time manifest. Its joint
reservation sums ordinary-family storage once, probe-family owner/scratch storage
once, six temporary read-only node views, coordinator metadata and two retained
reports. It does not double-count the ordinary-family profile carried as metadata
by `ProbePlan`.

Each request charges its full worst case before clock, identity or provenance
validation: 18 retained-node clock comparisons and two-sided visits to all three
velocity components for six branches. Attempts and arithmetic are bounded and
checked at admission. Refusals consume an attempt but leave both owners and the
binding schedule unchanged. The schedule advances only after all six branches
produce either a provenance/equality record or explicit missing evidence. An
in-progress numerical or provenance failure terminates the binder.

## Bounded example and limits

Run:

```sh
cargo run --release -p nsbu-benchmarks --example v2_node_binding
```

The N=[4,8,12], steps=[64,32,16], fixed M=12 startup example drives probe clocks
0, 63 and 128. Binding ordinary clock 0 after probe lookahead to 63 shows rest
evicted for branches 0, 1, 2 and 5 while branches 3 and 4 still retain bitwise
equal rest nodes. At ordinary clock 128 all six exact endpoint nodes are retained
and bitwise equal. Missing evidence is never reported as equality or zero.

This bridge supplies finite provenance records for a future diagnostic report.
It does not combine family identities, compare reconstructed derivatives, add a
policy tolerance, produce a numerical bound or qualify a PDE window. P08/P09/P10
remain incomplete and accepted concentrating windows remain zero.

## Focused verification

```sh
cargo test -p nsbu-benchmarks --lib v2_experiment::binding::tests
cargo test -p nsbu-benchmarks --test v2_node_binding
cargo test -p nsbu-benchmarks --test v2_node_binding_allocation
```

Actual from-rest tests cover retained equality at clocks 0, 64 and 128; mixed
branch eviction after lookahead; immutable coefficients, clock, epoch,
accepted-step, history and work records; invalid admission identity/cap/work;
stale and foreign owners; terminal probe state; exhaustion; schedule retention;
and allocation-free execution. No analytical state or interpolant replaces an
unavailable accepted node.
