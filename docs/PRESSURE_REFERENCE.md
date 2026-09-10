# Exact-v2 periodic reference pressure

`reference.pressure_gauge` computes the global spatial mean of the frozen
`similarity-mms-v2` raw pressure on the centered unit periodic cube. Reported
pressure subtracts that one constant everywhere, including outside the compact
support of the raw field. Regional error comparisons must use the same global
gauge; they must not subtract a separate regional mean.

This is independent Python reference evaluation. It does not evolve a PDE,
change a prescribed force, assign a reference into integrated state, or qualify
a concentrating window. The existing pointwise derivative fixtures intentionally
retain their original raw-pressure convention and immutable historical evidence.

## Run a bounded refinement study

Install the documented Python 3.12 development requirements, then run:

```sh
python -m reference.verify_pressure_mean --time 1/256 --panels 32 --dry-run
python -m reference.verify_pressure_mean --time 1/256 --panels 32 > work/pressure-mean.json
```

The time is an exact rational with `0 <= t < 1/128`. A base even panel count n
produces five geometries: (n,n), (2n,2n), (4n,4n), (2n,4n), (4n,2n), each at 80
and 120 decimal digits. The axes denote axial z and radial-squared w quadrature.
Every profile is admitted before any quadrature starts; all ten run sequentially.
Each panel count lies in [2,1024], so the command's base count cannot exceed 256.

The dry-run verifies the exact frozen case hash, prints every profile, the total
finite evaluation/root-work bounds and peak numerical reservation. The default
cap is 64 MiB. The reservation includes streaming scalar temporaries and a
conservative interpreter allowance; Python does not provide a hard allocator
ceiling. No full grid or all-node root table is retained. Each axial slab solves
one scalar root with a cap of 2048 iterations. Root exhaustion or nonfinite
quadrature fails the command without publishing a partial result.

Exit 0 means a completed preflight or empirical diagnostic, exit 1 means a
resource/input/evaluation refusal, and exit 2 means invalid command syntax.
The result preserves every mean, separate 80/120 arithmetic differences, joint
quadrature changes and both single-direction changes. Ratios with zero
normalizers are null; exact startup zeros do not establish a numerical floor.

## Integral and gauge convention

The frozen outer support radius is **21/50**, strictly inside the unit cube's
half-width. With w=x²+y², angular integration gives dx dy = π dw. Thus the raw
pressure volume average equals:

```text
π ∫[-Rout,Rout] dz ∫[0,Rout²-z²] p_raw(sqrt(w),0,z,t) dw
```

The unit cube has volume one. Both signs of z are integrated; no assumed evenness
removes half the domain. Axisymmetry reduces this reference integral only. The
solver still evolves all three velocity components on a Cartesian 3D grid.

The numerical radial integral starts at `a=max(0,Rin²-z²)`. In the core [0,a],
the spatial cutoff is exactly one, so its Gaussian integral is evaluated as:

```text
-c_t² q^(-1/4) [1-exp(-a/q)] / 32
```

The remaining cutoff collar uses composite Simpson quadrature. The axial
integral also uses composite Simpson quadrature. This preserves the exact
benchmark while removing its narrowing Gaussian core from radial sampling.
The original uniform full-radius pilot is retained as a resolution diagnostic;
precision alone did not fix its substantially larger quadrature change.

`PressureMean` binds the computed constant to its immutable `MeanPlan` and exact
time. Its `subtract(raw_pressure)` operation uses the declared precision; callers
must supply the raw pressure from that request. It has no state mutation or
force-provider interface. Spatial derivatives and pressure differences are
unchanged by this global constant. After subtraction, reported pressure outside
the raw support is generally nonzero; setting it to zero would use the wrong
gauge.

## Verification and limits

Tests check exact cubic quadrature and fourth-order quartic refinement, the
complete spherical volume/Jacobian, independent Cartesian scalar pressure,
convergence of separately sampled scalar quadrature to the exact-core formula,
startup zero, global gauge behavior, precision restoration and bounded failures.
The public command has real preflight, diagnostic, resource-refusal and invalid
argument tests, and CI runs its dry-run alongside the full reference suite.

All estimates and refinement differences are **empirical quadrature**. They are
not interval enclosures, certified continuum means or accepted PDE tolerances.
The result must be included in the reference-error budget at every tested time
used by a future concentrating comparison. Connecting that budget to complete
regional/reference observations and the window verifier remains implementation
work. Accepted concentrating PDE windows remain zero.
