# Independent arithmetic checks for physical fields

`reference.compare_derived` compares the actual Rust velocity, gradient, full
Hessian, vorticity, physical pressure and pressure gradient with independently
computed 80/120-digit fields. It measures signed component differences at every
physical sample and retains complex reference defects. Velocity-coefficient
agreement alone is insufficient for these derivative and pressure diagnostics.

This is a bounded smooth `CyclicSine` study on N=4, 8 or 12. It does not qualify a
concentrating trajectory, larger grids or a continuous interval. The existing
[arithmetic study](ARITHMETIC_STUDY.md) specifies the mathematical problem,
independent from-rest evolution, exact input-bit schema and stage schedule.

## What is computed

The Rust `smooth_derived` example evolves the same actual eight full/two-half
macro proposals as `smooth_coefficients`, committing sixteen independent fine
updates from rest. Both exporters share the admitted evolution helper. Their
state words agree exactly for the tested profiles; the reference never resets
or replaces the integrated state.

After reaching tick 128 at quantum `2^-16`, the exporter uses separate diagnostic
scratch to sample 46 ordered scalar fields on `(2N)³`:

| Quantity | Ordered components | Relative denominator floor |
| --- | ---: | ---: |
| Velocity | 3 | `1e-8` |
| Velocity gradient | 9 | `1e-7` |
| Velocity Hessian | 27 | `1e-6` |
| Vorticity | 3 | `1e-7` |
| Physical mean-zero pressure | 1 | `1e-8` |
| Pressure gradient | 3 | `1e-7` |

These are fixed smooth diagnostic floors, not qualified concentrating tolerances.
Both mixed Hessian entries are included. Each field retains its signed binary64
values, component index and Cartesian derivative counts. The full state and
unprojected pressure force are also exported as exact bits.

The Python implementation uses full-complex separable direct DFT sums, independent
of the Rust FFTs. It constructs physical pressure from all conservative tensor
products on the complete doubled band. No projected force or analytical pressure
is substituted. The global zero mode fixes the pressure gauge. Although exact
smooth pressure is zero, each numerical trajectory's reconstructed pressure can
be nonzero and those errors remain visible.

## Reproduce a comparison

First generate the force and four high-precision trajectories with the complete
[existing workflow](ARITHMETIC_STUDY.md#run-the-complete-comparison). For the first
pass, use `study_grid=4`, leaving its `work/cyclic-n4` directory intact. Then run:

```sh
study_grid=4
study_dir="work/cyclic-n${study_grid}"

cargo run --release -p nsbu-benchmarks --example smooth_derived -- \
  "$study_grid" CM --dry-run
python -m reference.compare_derived --n "$study_grid" --method CM --dry-run

for study_method in CM HO; do
  cargo run --release -p nsbu-benchmarks --example smooth_derived -- \
    "$study_grid" "$study_method" > "$study_dir/derived-${study_method}.json"
  python -m reference.compare_derived --n "$study_grid" --method "$study_method" \
    --rust "$study_dir/derived-${study_method}.json" --force "$study_dir/force.json" \
    --independent80 "$study_dir/independent-${study_method}-80.json" \
    --independent120 "$study_dir/independent-${study_method}-120.json" \
    --fixed80 "$study_dir/fixed-${study_method}-80.json" \
    --fixed120 "$study_dir/fixed-${study_method}-120.json" \
    > "$study_dir/derived-comparison-${study_method}.json"
done
```

Require every command to succeed before reading its output. A failed writer can
leave partial JSON, which is not a successful artifact. Python syntax failures
exit 2; bounded input/resource or numerical refusals exit 1 with diagnostic JSON.
A completed comparison exits 0 and retains `accepted_pde_windows: 0`.

Repeat with N=12 and independently generated N=12 inputs for the finest smooth
family grid. Reusing N=4 evidence does not supply current-grid arithmetic at N=12.
Direct high-precision sums can take many minutes; independent processes require
joint resource admission. The Python dry-run requires no artifact paths.

## Interpret the seven comparisons

| Report key | Meaning |
| --- | --- |
| `diagnostic_80_120` | Same exact Rust state and unprojected endpoint force; only independent diagnostic precision changes |
| `rust_vs_same_state_120` | Actual Rust diagnostic/FFT effect relative to independently sampled identical inputs |
| `fixed_80_120` | Independently evolved trajectories and diagnostic arithmetic at 80/120 digits, with the same exact force bits |
| `rust_vs_fixed_120` | Actual integration plus diagnostic arithmetic relative to the independent same-input trajectory |
| `independent_80_120` | Independent target-force trajectories and physical diagnostics at 80/120 digits |
| `fixed_vs_independent_120` | Prescribed-force approximation effects, with both trajectories and diagnostics computed at 120 digits |
| `rust_vs_independent_120` | Combined Rust discrepancy relative to the independent target-force trajectory |

For every quantity, the report contains RMS error, sampled absolute error peak,
reference peak and maximum pointwise relative error with the fixed floor. RMS
divides by physical points, not tensor entries. It accumulates the complete field
difference at each point; opposite equal-magnitude vectors cannot produce a false
zero. A failed partial reducer cannot publish a result or silently retry.

`precision_ratios` compares observed 80/120-digit RMS/peak changes with the
corresponding Rust discrepancy. A zero denominator is explicitly undefined;
it does not establish a supported error floor. The report separately preserves
maximum imaginary reference components, product leakage on excluded Nyquist
planes, and the original stored-state reality defect. No conjugate-pair repair,
alignment or common-band crop is used to improve agreement.

These ratios are empirical separation measurements. They are not rigorous
roundoff enclosures or an automatic window-acceptance decision.
The reported error norms are reduced in Python at 120 digits from the exported
scalar values. They do not separately measure Rust's physical error-reduction
arithmetic; that requires an additional comparison of the production reducer.

## Format, memory and finite work

The Rust schema is `NSBU_DERIVED_1`. Its sample lattice is unshifted `[0,1)³`, with
z varying fastest. For each velocity component, the rows are value, x/y/z first
derivatives, then all ordered first/second coordinate pairs for the Hessian.
Three curl components and pressure value/x/y/z derivatives follow. The reader
requires the entire exact order and every sample; missing fields and nonfinite
words are refused. Pressure forcing is read without projection, and must equal
the fixed endpoint input padded to the doubled band for this smooth profile.

Rust jointly admits the complete owned trajectory and export scratch before
allocation. The default cap is 64 MiB. Export uses **55 additional scalar
transforms**: 39 ordered velocity entries, three curl entries, four pressure
entries and nine conservative-product transforms. The JSON field
`diagnostic_scalar_transforms` refers to this export work; the owned trajectory's
integration and accepted-history work has its own admitted counters. Writers and
stored artifact buffers are caller-owned, outside the numerical reservation.

Python bounds each input read at 16 MiB and defaults to a 32 GiB planning cap.
Its dry-run itemizes parser, imported-field, current-field, mixed-derivative cache,
pressure/direct-DFT and complete reduction reservations. These are conservative
Python planning allowances, not hard allocator guarantees. It computes six
isolated precision profiles with **46 direct scalar transforms per profile**;
repeated mixed entries reuse their identical computed samples while retaining
full tensor multiplicity. Processes never share a mutable mpmath precision context.

## Source and checks

| Source | Responsibility |
| --- | --- |
| [Rust exporter](../crates/nsbu-benchmarks/examples/smooth_derived.rs) | Joint admission, tested argument path, actual from-rest export |
| [Rust diagnostic scratch](../crates/nsbu-benchmarks/examples/derived_support/mod.rs) | Independent pressure/curl and full domain binding |
| [Rust serialization](../crates/nsbu-benchmarks/examples/derived_support/output.rs) | Streaming exact words and ordered scalar fields |
| [Direct field mathematics](../reference/derived/spectral.py) | Complete derivatives, conservative pressure and retained complex defects |
| [Artifact reader](../reference/derived/schema.py) | Exact inventory, state and unprojected-force binding |
| [Arithmetic comparison](../reference/derived/comparison.py) | Preflight, isolated precision streams and separated error channels |
| [Field reductions](../reference/derived/reduction.py) | Complete per-point tensor differences and failure retention |

```sh
cargo test -p nsbu-benchmarks --example smooth_derived --example smooth_coefficients
python -m pytest reference/tests/test_derived_spectral.py \
  reference/tests/test_derived_contracts.py reference/tests/test_derived_comparison.py
```

Independent Taylor–Green coefficients and physical derivative checks cover
pressure sign/gauge, fine-only force modes and every ordered tensor entry.
Negative controls include incomplete/reordered fields, nonfinite words, changed
force, invalid floors/caps, failed partial reductions and invalid CLI arguments.
Complete concentrating observables, reference/force/current-grid studies,
accepted-artifact binding and continuous-window qualification remain in progress.
