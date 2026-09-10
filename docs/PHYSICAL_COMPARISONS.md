# Complete physical-field comparisons

`PhysicalComparisonWorkspace` compares two complete Fourier fields on a common,
unaligned physical lattice. It measures scalar values/gradients, velocity,
velocity gradients, the full Hessian and vorticity. Inputs are borrowed and can
come directly from two independently integrated trajectories.

The component is a numerical diagnostic. The owning experiment must separately
bind the mathematical problem, exact comparison time, from-rest origin and
reference/force/arithmetic evidence. A successful comparison does not accept a
PDE window.

## Fields and quantities

The scalar/vector input type must match the selected quantity:

| Quantity | Input | Ordered output components | Scalar inverse FFTs, both sides |
| --- | --- | ---: | ---: |
| `Scalar` | Scalar spectrum | 1 | 2 |
| `ScalarGradient` | Scalar spectrum | 3 | 6 |
| `Vector` | Three spectra | 3 | 6 |
| `Gradient` | Three spectra | 9 | 18 |
| `Hessian` | Three spectra | 27 | 54 |
| `Vorticity` | Three spectra | 3 | 12 |

Gradients use component/coordinate order; Hessians use
component/first-coordinate/second-coordinate order. Both mixed entries are
included in the Frobenius norm. Curl uses the physical orientation and two
independently sampled Cartesian derivatives per output component. All derivatives
use the actual physical lengths.

The two sources must have equal physical lengths and viscosity. The second grid
must retain the first grid's entire strict band, and the diagnostic sample grid
must be componentwise at least as fine as both. Every retained mode participates,
including modes that exist only on the finer grid. The implementation does not
align fields, project them or remove their means. A scalar pressure comparison
therefore preserves any supplied mean discrepancy; pressure construction and a
consistent physical gauge are separate responsibilities.

## Reservation and lifecycle

Call `PhysicalComparisonWorkspace::reservation(left, right, samples)` before
allocation. The constructor refuses a smaller cap before creating either scalar
sampler or the comparison arrays. Reservation includes owned elements and object
headers; caller spectra, stored reports and allocator overhead have separate
budgets.

The workspace owns two reusable scalar derivative samplers and four physical
arrays: the current component from each side, complete difference magnitudes and
complete reference magnitudes. Components are processed sequentially. Storage
is independent of whether the field has 1, 3, 9 or 27 ordered components.
No complete physical Hessian tensor is retained at every lattice point.

A successful `compare(left, right, quantity, relative_floor)` returns a borrowed
`PhysicalComparison`. Its view prevents scratch reuse until released. It exposes
both source domains, the sample layout, quantity, actual fixed transform count,
every difference/reference magnitude and the global error report. A failed call
returns no view; it can change scratch but leaves both inputs untouched. A later
valid call resets the accumulators and can reuse the workspace.

`compare_domains([left_domain, right_domain], ...)` reuses those samplers for
another source-grid pair. It checks the full alternate reservation and geometry
before rebinding diagnostic inputs. Physical lengths, viscosity and the sample
grid stay fixed, and owned storage cannot grow. The returned view records the
actual alternate domains. Plain `compare` always uses the constructor domains,
including after a refused alternate call. This permits sequential spatial and
temporal pairs without allocating a separate tensor workspace for every pair.

Calls allocate nothing after construction. The stated transform count is for
one completed call, not a finite allowance on the number of caller requests.
The owning experiment must budget its complete observation schedule and charge
failed attempts as required by its execution plan.

## Error semantics

At each point, accumulate the magnitude of the **complete field difference** and
separately the complete reference magnitude. Equal magnitudes of two opposite
vectors must not imply zero error. The global report uses the usual RMS over
points, sampled absolute peak, sampled reference peak and maximum pointwise
relative error with an explicit positive denominator floor.

`TensorErrors<C>::push_magnitudes` accepts already computed nonnegative finite
field-difference/reference magnitudes for sequential tensor producers. It retains
the full component count and the existing transactional refusal behavior. It
cannot establish the provenance of externally supplied magnitudes. Its first
argument is never the difference of two field magnitudes.

These results remain floating sampled measurements. Physical-grid, arithmetic
and reference refinements are required by the complete experiment protocol;
no sampled maximum is advertised as a rigorous supremum.

## V2 regional aggregation

`nsbu_benchmarks::regions::physical::measure` passes every complete comparison
sample through the existing v2 geometric partition. It requires unit-cube,
viscosity-one geometry, the exact v2 clock and finite root/attempt budgets. It
preserves the same component inventory and relative floor. The global statistics
are identical to the input comparison's global statistics; core, annulus,
remaining interior, collar and exterior statistics accompany them.

This adapter does not infer a time or mathematical problem from the spectra.
The caller-supplied clock must be bound to the actual fields by the experiment.
Geometric `RegionEmpty` and coverage quadrature remain separate from `NoSamples`.
A regional report cannot suppress a failed global or collar comparison.

## Verification commands

```sh
cargo test -p nsbu-solver --test physical_comparison --test tensor_errors
cargo test -p nsbu-benchmarks --test regional_physical --test integrated_physical -- --nocapture
```

Tests compare all quantities against independent single-wave derivatives on
non-unit domains, including a mode absent from the coarse grid. Negative controls
cover opposite vectors with equal magnitudes, mean pressure errors, malformed
later tensor components, incompatible source kinds/grids, bad floors and
insufficient aggregate memory. A signed curl fixture checks orientation. Isolated
allocator tests cover admission, refusal, complete sampling and regional reduction.

The integrated comparison test evolves separate CM and HO smooth trajectories
from rest, compares actual velocity/gradient/Hessian/vorticity fields and verifies
that both Fourier-state digests stay unchanged. Its small smooth errors are
retained as diagnostics. Complete concentrating all-observable production,
protocol/provenance binding and PDE qualification remain in progress.

The [six-trajectory physical family](PHYSICAL_REFINEMENTS.md) binds this diagnostic
to actual synchronized smooth states, a fixed quantity/floor inventory and a
finite measurement allowance. It produces spatial, temporal and method findings;
complete all-channel and concentrating qualification remain separate.
