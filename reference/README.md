# Independent numerical reference (P00B complete)

This Python package evaluates the exact v2 manufactured field and supplies small
arbitrary-precision operator and time-step fixtures. It is independent of the
Rust runtime. Run from the repository root after installing requirements:

```sh
python -m unittest discover -s reference/tests -v
python -m reference.verify_fields > work/reference-fields.json
python -m reference.verify_steps > work/reference-steps.json
python -m reference.verify_trajectory > work/reference-trajectory.json
python -m reference.verify_sampling > work/reference-sampling.json
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

The [smooth trajectory study](../fixtures/reference/smooth-trajectory.json)
evolves both methods independently from rest to `t=1/32`, with four time-step
refinements and an 80/120-digit comparison. The last measured H1 orders are about
4.00 (CM) and 4.01 (HO). This separate smooth case is not the concentrating case.
The [region study](../fixtures/reference/regions.json) checks mathematical
interior-volume coverage with explicit empty-region status and empirical Simpson
refinements; it does not provide rigorous quadrature enclosures.

`verify_sampling` evaluates the exact v2 force on grids 4, 8 and 12 and compares
full fine-band coefficients at startup and the first endpoint. It also compares
80 and 120 digits on the finest grid. This can take many minutes. Its results are
diagnostic; it has no authority to qualify a force-resolution channel or PDE window.

The N=8 and N=12 80/120-digit full-step comparisons also passed. Their full
coefficient payloads are preserved with deterministic gzip compression:
[N=8](../fixtures/reference/steps-n8.json.gz) and
[N=12](../fixtures/reference/steps-n12.json.gz). The matching
[N=8 report](../evidence/reference-steps-n8.json) and
[N=12 report](../evidence/reference-steps-n12.json) record compressed and
uncompressed SHA-256 hashes. Python’s standard `gzip` module can read them.

The [force-sampling study](../fixtures/reference/force-sampling.json) completed:
80/120-digit differences are below 3e-78 per coefficient, while full fine-band
H1 differences grow across these small sampling grids. Spatial resolution remains
unqualified. [Execution provenance](../evidence/reference-sampling.json) preserves
the exact executed source hashes.

P00B numerical and quality gates passed locally and in hosted CI. See the source-hashed
[progress report](../evidence/reference-progress.json) and
[quality measurements](../evidence/quality-reference/summary.json). No production
Rust integration or accepted PDE window is supplied by those P00B fixtures.
The separate [CyclicSine arithmetic workflow](../docs/ARITHMETIC_STUDY.md) now
compares actual owned Rust trajectories with independent 80/120-digit direct-DFT
runs on the same grid, holding raw force bits fixed in a separate study.

The [derived-field workflow](../docs/DERIVED_ARITHMETIC.md) extends these
comparisons to complete velocity gradients/Hessians, vorticity and mean-zero
physical pressure and its gradient. It retains all ordered tensor entries and
full doubled-band pressure, including imaginary and Nyquist-product defects.
`python -m reference.compare_derived --n 12 --method HO --dry-run` reports the
bounded input and numerical reservations before reading any artifacts. Actual
comparisons require the Rust exporter and four independently evolved trajectories;
the guide documents all six inputs and the seven separately reported effects.

## Concentrating diagnostic now available

`python -m reference.pilot` runs the exact-v2 N=4 CM trajectory from rest to
1/256 at 80 digits. It can take about twenty minutes. `pilot(4,8192,80,"HO",1024**3)`
from `reference.pilot` runs the independently evolved HO branch. The supported
divisors are 8192, 16384 and 32768; the existing reference grids/precision policy
and a complete force-cache reservation are enforced before evolution.

[The executed diagnostic](../evidence/p00c/README.md) reaches the first endpoint
with both methods, but retained-band tracking errors are large and spatial/force
sampling remains unresolved. No accepted PDE window follows from this run.

`python -m reference.verify_derivatives > work/derivatives.json` reproduces the
nine-point 80/120-digit physical derivative report, including all ordered velocity
gradient/Hessian entries, vorticity and pressure gradient. The public generator
and its Rust fixture synchronization are tested. This is pointwise arithmetic
evidence; those pointwise fixtures retain raw pressure. The separate global-gauge
study below supplies empirical mean refinements; no PDE window is qualified.
See [physical derivative diagnostics](../docs/DERIVATIVE_DIAGNOSTICS.md).


The [periodic pressure reference](../docs/PRESSURE_REFERENCE.md) adds a separately
bounded global-mean quadrature study with an exact Gaussian core and independently
refined axial/collar sampling. `python -m reference.verify_pressure_mean --dry-run`
prints the complete ten-profile admission; omit `--dry-run` to execute it. Original
pointwise fixtures retain raw pressure. Global gauge refinements are empirical
and do not qualify a PDE window.

The [production reduction audit](../docs/REDUCTION_ARITHMETIC.md) independently
checks complete scalar/vector/tensor statistics at 80/120 digits. It separates
component-to-magnitude effects from accumulation of already-rounded magnitudes.
`python -m reference.verify_reductions --n 4 --dry-run` admits the complete study
without input files; the guide supplies actual commands and fixture provenance.

## Exact-v2 current-grid reference arithmetic

The [guarded 12-cubed study](../evidence/p09/reference-arithmetic-full/README.md)
compares velocity, gradient, ordered Hessian and curl at ticks 0, 64 and 128.
It separates 80/120-digit refinement, rational versus binary64 coordinates,
and Rust reference-evaluator arithmetic. The recorded full run took 28 minutes;
it is an optional numerical study, separate from the fast guard tests and PDE
integration.

From the repository root with the Python development environment active:

```sh
mkdir -p work
cargo test -p nsbu-benchmarks --locked --test v2_reference_arithmetic_pilot --no-run
```

Set `producer_binary` to the executable path printed by Cargo. On Linux, run
with the same 45-minute time limit and 256 MiB address-space limit per process:

```sh
producer_binary=/absolute/path/to/the/printed/test/executable
timeout --kill-after=5s 2700s prlimit --as=268435456 \
  python -m reference.reference_arithmetic_full "$producer_binary" \
  > work/reference-arithmetic-full.jsonl
```

Other platforms need equivalent process limits. A complete run must end with
an `event: complete` record for all 5,184 rows; a progress record alone is not
completion. The retained precision diagnostic has its own pass/fail field.
The report provides empirical reference-evaluator differences, excluding
pressure/gauge, integrator arithmetic, continuum bounds and PDE qualification.
