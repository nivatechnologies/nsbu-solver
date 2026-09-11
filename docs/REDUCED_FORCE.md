# Force values from three-variable potentials

`nsbu_benchmarks::reduced_force::evaluate` evaluates the exact viscosity-one
`similarity-mms-v2` prescribed force using Taylor variables `w=x*x+y*y`, `z`, and
physical time. It retains total degree three, with 20 coefficients and 84 valid
convolution products. The existing Cartesian evaluator retains degree four in
four variables, with 70 coefficients and 495 products, because it also supplies
spatial force gradients.

This experimental evaluator is separate from both default force providers.
It changes floating arithmetic order. Its outputs need their own accuracy and
artifact identity when used in a numerical experiment. It does not evolve a
trajectory, reduce the dimension of the PDE, approximate the prescribed problem
by a surrogate, or supply force gradients.

## Working commands

```sh
cargo run --release -p nsbu-benchmarks --example reduced_force_profile -- --dry-run
cargo run --release -p nsbu-benchmarks --example reduced_force_profile
cargo test -p nsbu-benchmarks --test reduced_force --test reduced_force_allocation
cargo test -p nsbu-benchmarks --lib reduced_force
```

The public profile evaluates a fixed 16-by-16-by-16 set of points at ticks
`1,4,2,1`, with quantum `2^-10` and target `8`. Before calculation it reports
65,536 pointwise calls, the scalar/formal iteration bounds, zero numerical heap
storage and an 8 MiB stack planning allowance. The allowance is not a measured
stack high-water mark. Both evaluators solve their root independently at every
active point; this profile does not include axial caches, workers or FFTs.

Each time includes separately timed Cartesian/reduced passes and a complete
pointwise comparison outside the timed region. The comparison must satisfy the
scaled diagnostic threshold of 5e-12 or the example fails. Checksums and timings are
printed; checksums are not error bounds. These measured differences against the
Cartesian binary64 path complement independent high-precision fixtures.

## Exact force identity

Write the already cut-off poloidal potential as `A(w,z,t)`, swirl coefficient as
`B(w,z,t)`, and raw pressure as `P(w,z,t)`. These names refer to the unchanged
[reviewed Cartesian potentials](design/COMPLETE_DESIGN.md), including the spatial
cutoff and startup ramp. Define

```text
C = -A_z
D = 2*A + 2*w*A_w
u = (x*C - y*B, y*C + x*B, D).
```

For viscosity one, define the radial, swirl and axial force coefficients

```text
R = C_t + C*C - B*B + 2*w*C*C_w + D*C_z
    - (8*C_w + 4*w*C_ww + C_zz) + 2*P_w
S = B_t + 2*C*B + 2*w*C*B_w + D*B_z
    - (8*B_w + 4*w*B_ww + B_zz)
Z = D_t + 2*w*C*D_w + D*D_z
    - (4*D_w + 4*w*D_ww + D_zz) + P_z
f = (x*R - y*S, y*R + x*S, Z).
```

These identities follow from the Cartesian curl, product and chain rules. No
radius division occurs, so the formulas remain regular on the axis. The term
`j0` in the prescribed potential remains present; the evaluator does not assume
reflection symmetry in `z`. A separate symbolic calculation checks all 12
Cartesian transport-component identities (three each for temporal, advective,
viscous, and pressure terms) and the velocity-divergence identity for arbitrary
smooth `A,B,P`, reducing by `w=x*x+y*y`; every checked difference is exactly
zero. The [evidence packet](../evidence/p09/reduced-force/README.md) preserves
the executed symbolic script and report, together with fixture provenance and
reproduction instructions.
This is an algebra check, not a PDE calculation.

Force values need third derivatives of `A`, second derivatives of `B` and first
derivatives of `P`. Degree three therefore retains the required derivatives.
Differentiating the final force would require additional information; this API
makes no force-gradient claim. The complete degree-four Cartesian evaluator
remains available for that purpose and for independent comparisons.

## Arithmetic and ownership

The implicit scalar equation uses the existing safeguarded root solver, capped
at 128 iterations. Three fixed formal Newton corrections produce its jet in
`w,z,t`. Remaining time comes directly from the exact tick clock, separately from
elapsed time; the implementation never obtains a tiny remaining interval by
subtracting two rounded times.

The polynomial algebra owns fixed stack arrays. It uses explicit finite checks,
positive-domain fractional powers and bounded exponential/binomial compositions.
Logarithmic rescaling recovers representable derivative coefficients when the
exponential constant underflows. Its polynomial magnitude majorant is not a
floating-error enclosure. Exact flat branches at rest and outside support are
retained. Tests check neighboring representable inputs around the logarithmic
underflow threshold, whose construction itself can round.

Potentials, Cartesian transport assembly and Taylor algebra have separate
modules. Results retain velocity, raw pressure, four momentum terms per component,
scalar-root diagnostics and all twenty implicit residual coefficients. These are
read-only evaluator results. No numerical state, checkpoint or reference-assignment
interface enters the calculation. A future force provider must separately reserve
sampling/FFT/worker storage and bind this arithmetic choice into its evidence.

## Pointwise evidence and remaining checks

The existing nine independent high-precision field fixtures pass. A further 84
cases use exact binary64 coordinate words and exact dyadic clocks, evaluated with
the independent Python Cartesian implementation at 80 and 120 decimal digits.
They cover the axis, both signs of `z`, interior/collar/boundary neighborhoods,
rest, very early startup and remaining intervals down to `2^-70`. The largest
80-to-120-digit scaled change is approximately `4.42e-79`; the largest measured
binary64 discrepancy across velocity, raw pressure, force and momentum terms is
approximately `3.60e-14`, using `abs(error)/(1+abs(reference))`. The regression
test requires this quantity below `5e-12`; that margin is a pointwise test
contract, not a bound on untested points or integrated trajectories.

The generator ran under a 256 MiB process address-space limit, with a declared
maximum of 168 evaluations and 2,048 scalar iterations per Python evaluation.
Its measured peak resident set was 28 MiB on the recorded Linux profile. These
pointwise fixtures do not bound force coefficients or integrated-state errors.

Direct monomial tests cover every truncated product and derivative. Mixed
compositions are compared with the separate Cartesian algebra. Nonfinite input,
flat support, periodicity, axial asymmetry and arithmetic-overflow controls pass.
An isolated allocator test includes the first and repeated/nonmonotone/refused
calls. Focused quality gates and clean-source fixture/test/profile reproduction
pass; the [execution report](../evidence/p09/reduced-force/README.md) separates
these results from the full current-source hosted checks required for publication.

P08/P09 and concentrating validation remain incomplete. No accepted concentrating
PDE window is supplied by this evaluator, its profile or its symbolic identity.
