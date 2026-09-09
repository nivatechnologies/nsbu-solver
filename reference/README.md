# Independent numerical reference (P00B in progress)

This Python package evaluates the exact v2 manufactured field and supplies small
arbitrary-precision operator and time-step fixtures. It is independent of the
future Rust runtime. Run from the repository root after installing requirements:

```sh
python -m unittest discover -s reference/tests -v
python -m reference.verify_fields > work/reference-fields.json
python -m reference.verify_steps > work/reference-steps.json
```

The field study compares degree-four implicit Taylor jets against independently
differentiated explicit scalar velocity formulas at 80 and 120 digits. It samples
axis, startup, inner/outer cutoff, collar and small remaining-time points. At
initial time, the independent temporal derivative uses a one-sided stencil to
stay in the declared time domain. A separate centered-difference refinement test
checks second-order derivative convergence. These are empirical checks, not
interval enclosures. Scalar and jet paths share the bounded scalar root and exact
rational conversion; the jet differentiation and scalar first-derivative formulas
are separate. Neither is a production kernel.

The DFT implementation uses normalized full complex coefficients, separable direct
sums, strict Nyquist removal, 3/2 padding and projection. An independent conservative
coefficient convolution checks the rotational grid product. Tests include mean
preservation, aliasing, energy, diffusion and nonmonotone HO stage-time requests.

The step study reserves memory before allocation and evolves an N=4 smooth forced
fixture from exact rest for one full step and two half steps, using CM and HO at
80 and 120 digits. The force is explicitly defined in its report and is not the
concentrating benchmark. The reservation is conservative diagnostic planning, not
a hard bound on Python allocator behavior. Reference steps allocate memory; they
do not implement the production transactional/bounded-attempt contract.

Committed fixtures are [field samples](../fixtures/reference/fields.json) and
[full-band step coefficients](../fixtures/reference/steps-n4.json). Regeneration
reports floating arithmetic changes explicitly; fixture regeneration alone is not
an independent correctness oracle.

P00B remains incomplete. Outstanding work includes N=8/12 arithmetic studies,
short refined smooth trajectories and temporal-order evidence, comprehensive jet
algebra/force-gradient and cancellation studies, region-coverage quadrature,
force-sampling studies, and the required code-quality gates. No Rust implementation,
current-grid binary64 arithmetic comparison, concentrating trajectory or accepted
PDE window is supplied by this package.
