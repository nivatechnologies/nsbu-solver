# Exact-v2 analytical trajectory tracking

This increment measures complete global sampled velocity, gradient, ordered
Hessian and vorticity errors for all six actual exact-v2 family branches. The
[public guide](../../../docs/V2_REFERENCE_TRACKING.md) defines its ownership,
resource, sampling and scientific boundaries.

Four focused harness tests, one magnitude unit test, one allocation executable
and the release example pass. The independent oracle reconstructs actual fields
through separate signed full-complex Fourier sums and checks RMS, absolute peak,
relative peak and reference peak for every branch and quantity. The existing
120-digit tensor fixture independently cross-checks the analytical Rust path.

At endpoint 1/8192, N12/h16 CM RMS errors are 1.8873998839615518e-7 for
velocity, 9.142791231564898e-6 for gradient, 9.325614204173285e-4 for the
ordered Hessian and 9.0193622044277e-6 for vorticity. The example log retains all
six branches at rest and both positive clocks. These are coarse sampled errors,
without a convergence or acceptance claim.

Focused LLVM coverage across six implementation/test files is 702/720 executable
lines (97.50%) and 21/24 branches (87.50%). The separately executed example is
included in static analysis without repeating it under LLVM. Across all seven
focused files, maxima are CC11, cognitive12, Halstead67.3654 and 353 physical
lines. Maximum per-function CRAP is 11. Focused strict Clippy and formatting and
whole-workspace Rustdoc pass.

Admission and execution allocate nothing. Construction used 20,135,360 bytes
within the 25,143,560-byte joint family/consumer reservation. The final profile
admits 5,184 analytical evaluations, 663,552 conservative root iterations, 810
inverse transforms and 4,920,528 weighted visits. Foreign/stale/terminal and
exhausted requests retain state, schedule and charged work.

During development, an all-channel `>1e-6` assertion failed on the measured
velocity RMS 1.8874e-7; it was corrected to preserve the actual channel scales.
Review also identified direct squared magnitude accumulation and one unchecked
intermediate work product. The final implementation uses finite checked `hypot`,
tests 1e-200/1e200 magnitude controls, uses checked work arithmetic and reserves
explicit allocator allowances.

Binary64 reference/current-grid arithmetic, pressure and gauge, regional
coverage, force/transfer sufficiency and artifact provenance remain pending.
Accepted concentrating windows remain zero.
