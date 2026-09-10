# Reproducing the smooth arithmetic study

This workflow measures arithmetic on the **same retained grid and accepted
fine-step schedule** used by the public `CyclicSine` experiment. It compares the
Rust implementation with independent Python direct-DFT trajectories at 80 and 120
decimal digits. Every trajectory starts at rest. The analytical field supplies
prescribed data and comparison values; it never replaces an evolved state.

The outputs are diagnostic measurements. They do not qualify a concentrating
endpoint, supply all benchmark observables, or authenticate an imported report.
The concentrating definition remains the byte-preserved
[`similarity-mms-v2.json`](../benchmarks/similarity-mms-v2.json).

## Fixed mathematical and execution profile

The periodic domain is the unit cube with viscosity one. The smooth analytical
velocity is

```text
u = (sin(13t) sin(2πy), sin(17t) sin(2πz), sin(19t) sin(2πx)).
p = 0.
f = ∂t u + div(u ⊗ u) − Δu.
```

Both implementations solve the projected velocity equation, including its full
nonlinear term. The reference constructs the prescribed projected force through
an independent sparse coefficient convolution. The Rust provider uses explicit
coefficient formulas. Neither implementation samples the evolving state to
construct the prescribed force.

| Setting | Value |
| --- | --- |
| Supported retained grid | N=4, 8 or 12 in every direction |
| Nonlinear product grid | 3N/2 in every direction |
| Exact clock quantum | 2⁻¹⁶ |
| Endpoint | tick 128, or t=1/512 |
| Macro step | 16 ticks, or 1/4096 |
| Accepted macro proposals | 8 |
| Evolved fine updates | 16, from two half steps per macro proposal |
| CM RHS evaluations | 96, including discarded coarse proposals |
| HO RHS evaluations | 120, including discarded coarse proposals |
| Distinct force stage clocks | 0, 4, 8, …, 128 |

The production owner applies its local-error and advective guards and must commit
all eight proposals before exporting a state. Absolute velocity/vorticity L2 tolerances are
0.01, relative tolerances zero, and the advective limit is 0.3. The Python
reference executes the fixed full/two-half schedule and records every proposal
difference. It is an allocating numerical oracle, not an implementation of the
production controller. It always advances with the independently computed fine
proposal, without Richardson extrapolation.

## Run the complete comparison

Use the repository's pinned Rust toolchain and Python 3.12 environment described
in [installation](INSTALL.md). `requirements-dev.txt` supplies mpmath 1.3.0.
Run from the repository root:

```sh
study_grid=4
study_dir="work/cyclic-n${study_grid}"
mkdir -p "$study_dir"

cargo run --release -p nsbu-benchmarks --example smooth_force_coefficients -- \
  "$study_grid" > "$study_dir/force.json"

python -m reference.verify_cyclic --n "$study_grid" --precision 120 --method HO \
  --force-bits "$study_dir/force.json" --dry-run

for study_method in CM HO; do
  cargo run --release -p nsbu-benchmarks --example smooth_coefficients -- \
    "$study_grid" "$study_method" > "$study_dir/rust-${study_method}.json"
  for study_precision in 80 120; do
    python -m reference.verify_cyclic --n "$study_grid" --method "$study_method" \
      --precision "$study_precision" \
      > "$study_dir/independent-${study_method}-${study_precision}.json"
    python -m reference.verify_cyclic --n "$study_grid" --method "$study_method" \
      --precision "$study_precision" --force-bits "$study_dir/force.json" \
      > "$study_dir/fixed-${study_method}-${study_precision}.json"
  done
  python -m reference.compare_cyclic --n "$study_grid" --method "$study_method" \
    --rust "$study_dir/rust-${study_method}.json" --force "$study_dir/force.json" \
    --independent80 "$study_dir/independent-${study_method}-80.json" \
    --independent120 "$study_dir/independent-${study_method}-120.json" \
    --fixed80 "$study_dir/fixed-${study_method}-80.json" \
    --fixed120 "$study_dir/fixed-${study_method}-120.json" \
    > "$study_dir/comparison-${study_method}.json"
done
```

Require each command to exit successfully before using its output. A failed
writer can leave partial JSON in a redirected file. Repeat with `study_grid=12`
to measure arithmetic on the finest grid of the six-trajectory smooth example.
The N=4 result alone is insufficient for that experiment. Direct high-precision
DFTs on N=12 can take tens of minutes per trajectory. Separate processes may run
independently after admitting their aggregate memory reservations.

The reference uses separable full-complex direct sums, without a Rust kernel or
FFT library. Each one-dimensional sum uses mpmath's dot-product accumulator to
retain small cancellation terms before rounding. Transform roots and subsequent
operations still use finite declared precision. The 80/120 comparison measures
the resulting change; it does not prove an interval enclosure.

## Interpreting the five comparisons

Every difference includes **all modes of the declared retained grid**, with unit
cube Fourier L2 and inhomogeneous H1 norms. H1 weights each squared coefficient
by `1 + 4π²|k|²`. No common-band truncation, alignment, divergence projection or
conjugate-pair averaging is applied to imported evolved states.
Each comparison also persists every signed complex coefficient difference to
40 decimal digits. Original 80/120-digit state coefficients and exact Rust words
remain in the separately identified input artifacts.

| Output | What changes |
| --- | --- |
| `independent_precision_80_120` | Reference arithmetic and independently evaluated forcing precision |
| `fixed_precision_80_120` | Reference arithmetic, holding every raw prescribed force bit fixed |
| `rust_vs_fixed_120` | Rust arithmetic versus the independent reference with identical raw stage inputs |
| `fixed_vs_independent_120` | Effect on the trajectory of binary64 prescribed-force evaluation |
| `rust_vs_independent_120` | Combined arithmetic and prescribed-force difference |

`stage_projected_force_maximum` separately measures the largest force coefficient
difference over all 33 stage clocks. `rust_reality_defect` reports floating-point
asymmetry on stored conjugate pairs. The comparison retains that asymmetry in the
state difference. Its negative-control test ensures it cannot silently repair a
state or omit an excited high mode.

## Input identity and resource admission

Rust exports unsigned 64-bit words for every real and imaginary component,
including exactly zero excluded Nyquist coefficients. The reference decodes the
integer mantissa and exponent directly, preserving subnormals without decimal or
binary64 conversion. The canonical storage order is x, y, then nonnegative z;
the excluded Nyquist coordinate is positive. Missing, duplicated or reordered
modes and clocks, nonfinite values, and nonzero excluded Nyquist entries fail
admission. Fixed force inputs additionally require exact Hermitian reality.

The force fixture is immutable through its public API. Its SHA-256 binds every
supplied byte. Both fixed-input reports must name that exact digest. The comparison
checks the grid, method, precision, endpoint, step schedule, RHS count, explicit
diagnostic status and every coefficient. A matching digest or internally
consistent report is **not proof of physical origin**. Arbitrary fixed inputs are
labeled `fixed-stage-arithmetic-fixture`; the comparator retains external
diagnostic origin and never produces an accepted window.

Each Python input is limited to 16 MiB, with at most 32 JSON container levels.
Repeated keys and nonstandard NaN/Infinity constants are refused. Before reading
a fixed input, the trajectory command admits its reference workspace plus a
768 MiB fixture/parser reservation. N=12 reserves 3,951,296,512 bytes within its
default 4 GiB cap. The comparator reserves 2 GiB before reading any input. These
are conservative Python planning reservations, not hard allocator guarantees.
The Rust exporters preflight their owned numerical storage under 64 MiB; output
is streamed to a caller-owned writer. The force producer evaluates exactly 33
arrays, and the state producer permits only eight attempts.

`verify_cyclic --dry-run` validates the complete fixed input and reports its
digest without allocating trajectory or DFT arrays. Python syntax errors exit 2;
input, I/O and resource refusals exit 1 with diagnostic JSON; successful preflight
or diagnostic execution exits 0. The Rust examples return a nonzero status on
invalid arguments, numerical refusals or write errors. A zero exit status is not
a PDE qualification.

## Code map and remaining verification

- [`profile.py`](../reference/cyclic/profile.py) constructs independent analytical coefficients.
- [`study.py`](../reference/cyclic/study.py) owns reference admission and the from-rest schedule.
- [`bits.py`](../reference/cyclic/bits.py) validates immutable exact-stage inputs.
- [`results.py`](../reference/cyclic/results.py) checks imported reports without repairing evolved states.
- [`comparison.py`](../reference/cyclic/comparison.py) separates the measured differences and retains input hashes.
- [`smooth_coefficients.rs`](../crates/nsbu-benchmarks/examples/smooth_coefficients.rs) exports the actual owned Rust trajectory.
- [`smooth_force_coefficients.rs`](../crates/nsbu-benchmarks/examples/smooth_force_coefficients.rs) exports the prescribed raw stage inputs.

These velocity arithmetic measurements are one part of P09. Complete observable
and time inventories, frozen channel tolerances, full experiment lineage,
pressure and regional diagnostics, force-resolution, sampling and quadrature
refinements remain necessary for P08/P09 and concentrating qualification. The
active [implementation plan](../IMPLEMENTATION_PLAN.md) and
[machine-readable status](../project-status.json) record demonstrated package exits.
The [executed arithmetic evidence](../evidence/p09/arithmetic/README.md) preserves
the N=4/N=12 runs, complete coefficient differences, source hashes and quality reports.
