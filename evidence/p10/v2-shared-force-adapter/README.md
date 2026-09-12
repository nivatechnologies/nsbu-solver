# Shared-force adapter evidence

This package is bound to executable source bfae37e4 and the documentation-only tip
7867daf. It preserves the earlier hardware-pause handoff separately, including the
user-interrupted exit 143, and does not rewrite that historical record.

The focused fixture drives six independently owned CM/HO N4/N8/N12 trajectories through
the existing generic SpectralRhs and recorded_step path. Two committed nonzero states per
trajectory match separate direct-force Run controls at every coefficient word. Outcomes,
complete histories, and independently sampled fresh M32 observer reports also match. The
shared integration table evaluates nine clocks and serves 162 exact ordered calls.

Joint admission owns one table and issues each admitted handle once. It binds force
settings, retained domain, full clocks, interval, and method-derived CM12/HO15 call order.
A conflicting RefCell borrow, malformed output, wrong limit/clock/order, incomplete
attempt, or exhausted cap terminates only according to the documented fail-closed path.
Detailed table refusal categories are preserved when converted to SolverError.

The current table manifest requires positive copy counts for all three domains at each
clock. The existing FamilyPlan geometry can meet that condition because all three spatial
branches share the finest step and extra temporal branches use N2. More general schedules
with a zero-use domain remain unsupported. Owned Run/V2Family scheduling, retry/slab
coordination, archive, and CLI integration are outside this increment.

Focused branch coverage is reported at its measured 69.12 percent. The required
production-plus-test CRAP maximum is 22.5. This evidence establishes a standalone
trajectory-equivalence bridge; it does not establish force accuracy, convergence, a PDE
window, or scientific qualification.
