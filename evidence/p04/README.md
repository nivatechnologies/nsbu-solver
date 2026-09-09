# P04 Cox–Matthews attempts and commits

The binary64 coefficient evaluator implements the reviewed nineteen-term small
argument series, intermediate closed forms and large-negative reciprocal formulas.
Independent 80/120-digit hypergeometric fixtures include both sides of branch
junctions, a weight zero and extreme negative arguments. Combined weights and
constant-source identities pass. Analytic truncation bounds exclude floating-point
roundoff; the checked absolute coefficient comparison tolerance is 4e-15.

The independent Python direct-DFT fixture evolves an N=4 cyclic-sine prescribed
source from exact rest for 1/128, with full and two-half CM steps. Its 80/120-digit
full-band change is 1.648e-83 or less. Rust's normalized FFT, rotational operator
and CM kernel agree with both retained-band results within absolute 1e-14 per
complex component. This is a smooth semidiscrete step check, not a converged PDE
trajectory or a concentrating benchmark result. The earlier 1/100 fixture was
not changed. [The execution recipes](fixture-generation.md) record provenance.

The one-step kernel uses five independently owned vector scratch slots and four
RHS evaluations at actual start, midpoint, midpoint and endpoint clocks. It checks
finite inputs, stage fields, RHS outputs and the final combination. A complete
attempt evaluates one full step and two half steps, proposes the fine state without
extrapolation, and compares raw velocity and curl discrepancies in volume-average
L2 norms. No Richardson divisor is used. Logical clocks remain exact u128 counts;
interval conversion refuses lost binary64 bits, underflow or overflow. Provider
clocks carry elapsed and remaining separately. The core never rounds or halves a
request. A separate scheduler halves only local-error rejections, has a maximum of
64 attempts, and propagates other failures without retry.

A successful local comparison issues a private, non-clonable acceptance token.
Commit validates the base field fingerprint, exact clock, epoch, plan, storage
pointers/lengths/capacities, accepted-step count, candidate generation and acceptance
identity before a single payload swap. The fingerprint supplements private buffer
identity; it is not the cryptographic mathematical-problem ID. The candidate takes
back the previous committed buffers. Count/generation overflow, stale tokens and
all twelve RHS failure positions preserve the committed state. Private-holder
corruption tests independently exercise each defensive check.

RHS implementations declare storage, finite provider work and scalar transforms;
undeclared costs cannot enter the bounded attempt interface. Prescribed-force
callbacks receive no evolving state or reference assignment interface. Providers
must enforce their internal declarations, including error paths: these interfaces
cannot impose a wall-clock limit on arbitrary external code. The spectral RHS
charges full declared work on failed calls, validates successful consumption,
refuses changed declarations and limits an attempt to twelve invocations. It
checks the sampled padded-grid advective guard and an exact remaining-tick fraction.
Physical pressure costs one additional transform per RHS: a completed CM attempt
uses 120 scalar 3D transforms before additional declared force transforms.

Resource preflight includes kernel arrays, coarse/midpoint vectors, coefficient
tables, operator storage, provider storage and explicit caller overhead. Two
independent allocation probes cover operator calls and twenty sequences of
numerical rejection, stale-token refusal and accepted swap commits. After setup,
these sequences allocate, deallocate and reallocate zero times. The allocation stress
probe deliberately uses permissive local tolerances for successful commits; it is
not convergence evidence. Allocation failure
and complete reservation formulas have separate tests.

SOLID review keeps coefficient mathematics, exact interval admission, local norms,
spatial/provider accounting, step assembly, attempt policy, external scheduling
and payload ownership separate. The kernel borrows a specific RHS trait interface;
it neither owns a reference evaluator nor allocates a dynamic callback container.
This also avoids asymmetric monomorphization accounting in LLVM summaries, described
in [the upstream coverage report](https://github.com/taiki-e/cargo-llvm-cov/pull/494).
The strict coverage gate remains unchanged; no source or tests were excluded.

[summary.json](summary.json) records complete source and fixture hashes, each quality
metric, raw reports and mutation outcomes. No non-equivalent survivor or timeout
is accepted; unviable compiler replacements remain separate from caught mutants.
LLVM regions and inactive instantiations are reported independently and are not
claimed as fully covered. No private Niva dependency or new runtime dependency is
introduced. Hosted verification remains pending until recorded here.

No Rust exact-v2 force provider, concentrating Rust trajectory, qualified PDE
window, numerical CLI, checkpoint or viewer is supplied by this package. Reference
access remains separate from integration. P05 supplies the independent Rust scalar
and jet implementation; later packages supply refinement and experiment acceptance.
