# Physical derivative diagnostics

P08 supplies read-only tools for comparing velocity gradients, Hessians and
physical pressure derivatives. They expose local errors that a small global
velocity error can miss. These tools are implemented and tested; their complete
concentrating experiment integration remains in progress.

## Sample an actual Fourier field

`nsbu_solver::diagnostics::derivatives::DerivativeWorkspace` samples a scalar
Fourier component on an explicitly selected physical grid. `Derivative::new`
admits a Cartesian multi-index of total order zero, one or two:

| Multi-index | Sampled quantity |
| --- | --- |
| `[0,0,0]` | Scalar value |
| `[1,0,0]` | First physical x derivative |
| `[0,2,0]` | Second physical y derivative |
| `[1,0,1]` | Mixed x/z derivative |

Apply the workspace successively to each velocity component to obtain the full
velocity gradient or Hessian. Apply it to a separately computed mean-zero physical
pressure spectrum for pressure and its gradient. Pressure construction remains
the responsibility of the conservative/rotational operator; sampling does not
construct pressure or choose a gauge.

The workspace uses the domain's actual lengths in `ik`, visits the complete
strict retained band, zero-pads onto a componentwise equal or finer grid and
performs one scalar inverse FFT per call. It does not project, remove a mean,
recenter or align a field. Nyquist, finite-value and Hermitian admission follow
the same spectrum contract as committed states. Retaining additional high modes
cannot silently turn the primary comparison into a common-band comparison.

```rust
use nsbu_solver::{
    diagnostics::derivatives::{Derivative, DerivativeWorkspace},
    domain::{Domain, Layout}, Complex64, SolverError,
};

fn mixed_derivative(
    domain: Domain, samples: Layout, component: &[Complex64], cap: usize,
) -> Result<(), SolverError> {
    let mut workspace = DerivativeWorkspace::new(domain, samples, cap)?;
    let field = workspace.sample(component, Derivative::new([1,0,1])?)?;
    for (index, value) in field.values.iter().enumerate() {
        let coordinate = field.point(index)?;
        // Pass the value and identical, unaligned coordinate to a read-only comparison.
        let _ = (coordinate, value);
    }
    Ok(())
}
```

`reservation(domain, samples)` reports all owned element storage and object
headers before allocation. The caller separately budgets its inputs, retained
results and allocator overhead. The workspace reuses one scalar physical buffer,
one padded spectral buffer and FFT scratch, allowing tensor components to be
processed sequentially. Calls after construction allocate nothing. A failure
returns no sample view and leaves the input unchanged; scratch is reusable on a
subsequent valid call. Live sample views borrow the workspace and prevent reuse
until the view is released.

The finite grid produces sampled values, not a certified maximum or continuum
integral. Diagnostic sample-grid refinement and arithmetic evidence remain
necessary, including for region boundaries and derivative-sensitive norms.

## Independent v2 reference derivatives

`nsbu_benchmarks::fields::reference::evaluate` returns analytical velocity,
all nine gradient entries, all 27 ordered Hessian entries, vorticity, raw pressure
and its three spatial derivatives at a fixed physical point and exact benchmark
clock. It accepts no evolving state and supplies no state-assignment operation.
The existing force provider's evaluation path is unchanged.

The Rust implementation uses its existing implicit jets and mathematical field
formulas. Independence for its accuracy check comes from the separate Python
implementation at 80 and 120 digits. The Rust reference alone does not satisfy
a current-grid reference/arithmetic channel.

Pressure is **raw** in this pointwise evaluator. Subtract one independently
measured periodic spatial mean before pressure tracking comparisons. Do not
subtract a separate mean in each local region: that would hide spatial errors.
Pressure gradients are gauge independent.

## Scalar, vector and tensor errors

`TensorErrors<C>` accumulates complete samples with `1 <= C <= 27` in fixed
storage. The existing `ErrorAccumulator` name remains the three-component alias.
`RegionalTensorErrors<C>` applies the same measurements to the complete declared
lattice and all v2 geometric classes; `RegionalErrors` remains its vector alias.
Reports retain the component count.

Flatten a velocity gradient in component/coordinate order and a Hessian in
component/first-coordinate/second-coordinate order. Include both mixed Hessian
entries: the Frobenius norm counts both `d_xy u` and `d_yx u`. For each sample,
the error is the Euclidean/Frobenius magnitude of the complete field difference.
RMS normalization divides the squared sum by the number of sampled points,
**not by the number of tensor components**. Relative peak error divides each
point's error by the larger of its reference magnitude and the declared positive
floor. Reports preserve that floor and the sampled reference peak.

No samples remains `NoSamples`, independently of mathematical `RegionEmpty`.
The core, annulus, remaining interior, collar and exterior reports accompany the
global result. Region coverage quadrature remains separate. Invalid data and
arithmetic failures preserve previously accumulated measurements. A tensor with
an error confined to the cutoff collar must still fail both the relevant collar
comparison and any applicable global budget.

## Reproduce the reference fixtures and tests

From the repository root with the documented Python dependencies installed:

```sh
python -m reference.verify_derivatives > work/derivatives.json
cargo test -p nsbu-solver --test derivative_sampling --test tensor_errors
cargo test -p nsbu-benchmarks --test reference_derivatives \
  --test regional_tensors --test integrated_derivatives -- --nocapture
python -m unittest reference.tests.test_derivative_fixtures -v
```

The generator performs nine fixed point cases at two fixed precisions. Its
46-field output includes origin, near-axis, startup, cutoff edges/collar and a
late remaining-time case. It reproduces
[`fixtures/reference/derivatives.json`](../fixtures/reference/derivatives.json);
Python tests verify the corresponding Rust TSV values exactly. The original
reviewed inputs and preexisting reference fixtures remain unchanged.

The new fixture reports the observed componentwise 80/120 change divided by
`max(1, abs(high_precision_value))`; it requires this change below `1e-60`.
Rust tests compare against the 120-digit values with their separately declared
binary64 tolerance. Additional Python tests differentiate the independent scalar
formulas for selected first, pure second, mixed second and pressure derivatives.

At the exact rational outer cutoff edge, finite-precision jet evaluation can
return exponentially tiny values because the computed cutoff argument rounds
just inside the transition. The fixture retains these values. Exact rest and
points strictly outside the support exercise exact-zero flat behavior. The late
non-dyadic rational test time uses the same explicitly rounded `2^-100` Rust clock
as the earlier P05 point fixtures; that conversion is part of the measured error.

The actual-trajectory test independently evolves CM and HO from rest to `1/512`
on N=4 with eight committed full/two-half proposals. It compares sampled first
and second derivatives with independent trigonometric reference formulas, retains
nonzero errors and checks that sampling did not modify Fourier state. This is a
small smooth diagnostic, not a qualified concentrating trajectory.
