# Exact-v2 accepted-node binding

This increment binds ordinary exact-v2 family endpoints to exact nodes still
retained by the independent reconstruction-capable probe family. The [public
guide](../../../docs/V2_NODE_BINDING.md) defines its private read-only node view,
provenance validation, lookahead eviction and scientific limits.

Three actual two-family tests, one signed-word unit control, one allocator
executable and the release example pass. At accepted clocks 0, 64 and 128, all
six nodes are bitwise equal when the probe family advances through the same
manifest. In the separate lookahead profile [0,63,128], ordinary rest is absent
from branches 0, 1, 2 and 5 after the clock-63 probe, while the h64 and h32
branches retain bitwise-equal rest. The same pattern remains at clock 64 after
probe lookahead to 128; all six endpoint-128 nodes are then retained and equal.
No later current state or interpolated field is substituted for a missing node.

The binder requires stored branch, owner/probe identity, from-rest origin,
domain, clock, epoch and accepted-step provenance before comparing coefficient
words. Malformed provenance is a refusal distinct from `Compared` with a false
`coefficients_equal` finding. Signed zero, a one-ULP finite change and length
mismatch independently produce false. False is retained evidence and is never
interpreted as an error bound or tolerance decision.

Nightly branch coverage across the three new production files is 235/248 lines
(94.76%) and 21/26 branches (80.77%). Across those files, the two minimally
extended owner files, tests and example, maxima are CC14, cognitive8,
Halstead66.9778 and 423 actual lines. Maximum CRAP in new production is 13.
Focused strict Clippy/formatting and the exact strict workspace Rustdoc gate pass.

Admission and execution allocate nothing. Constructing both six-owner families
used 40,551,040 bytes within the 50,555,344-byte joint reservation. Three
requests charge 54 worst-case ring-clock comparisons and 79,200 two-sided
coefficient visits. Refused stale, foreign, terminal and exhausted requests
consume their declared attempt where applicable while preserving both owner
states and the binding schedule.

The first source-matched coverage report showed 64.71% branches in the binder
because six repeated result exits and a long provenance condition created
structurally uncovered branches. The final implementation uses one checked
branch loop and one observed-versus-expected provenance record, reaching 80.77%
without weakening validation. The final stored report was regenerated after that
change. An initial coverage command omitted the separator before the unit-test
filter and was rejected before compilation; the successful command is the one
retained.

The bridge emits provenance and raw equality/missing records for later diagnostic
composition. It does not merge family identities, measure derivative equality,
create a numerical bound, select a tolerance, establish convergence or accept a
PDE window. Accepted concentrating windows remain zero.
