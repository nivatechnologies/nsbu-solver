# Exact-v2 spectral force-resolution binding

`v2_force_experiment::binding` joins an already evolved `ForceFamily` to one
independently evolved baseline branch of an ordinary `V2Family`. It owns no
trajectory. Admission requires identical exact manifests and complete matching
run settings for the selected branches, then counts both family reservations,
the constant-storage binder and every coefficient-word comparison under one cap.
That joint value covers only the `ForceFamily`, the plain ordinary `V2Family`, and
the binder. A caller borrowing `DiagnosticDriver::ordinary()` must separately add
the driver's probe family and diagnostic consumers, subtracting the ordinary
family already counted in the driver before composing bounds. The binder value is
not a complete coordinator reservation.
The charged coefficient-word count is the conservative full allowance charged at
the start of every attempt, including failures detected by identity, schedule or
settings checks before coefficient traversal. The attempted-report count is
retained separately; neither field claims to be a hardware counter of visits.

At each clock both complete families must have successfully published the expected
sample. The binder checks case and family identities, exact clock and settings,
then compares every real and imaginary coefficient word in all three baseline
components. A shared identity or matching selected-state clock is insufficient.
A foreign, stale, partial or terminal owner fails the charged attempt. The prior
successful report remains available, the binder terminates, and no partial new
report is published.

The immutable `SpectralForceResolutionSample` retains the raw M0/M1 and M1/M2
`BandComparison` values, both family identities, case hash, settings and baseline
slots. These are spectral norms: `full.l2` is the velocity volume-average L2 norm,
`full.vorticity_l2` is curl L2, and `full.h1` combines velocity with all first
derivatives. They are not sampled physical gradient or Hessian RMS values and are
not mapped to the separate partial review adapter's observable keys. Status is
permanently `DiagnosticOnly`; no budget decision or PDE-window acceptance occurs.

The focused profile evolves N4 CM trajectories from rest to tick 128 with
M8/M16/M32 force grids. Its M16 branch is bitwise bound to the ordinary family's
N4/M16 finest-step CM branch. Independent signed-Fourier sums check both raw
force-resolution comparisons. Negative controls cover mismatched plan slots,
foreign identity, an unadvanced ordinary owner, a partially advanced force owner,
a terminal force owner, one corrupted coefficient word, attempt exhaustion and a
one-byte-short joint cap. Planning and steady binding allocate nothing.

This binder measures only one force-resolution input channel. It does not measure
force precision, physical/reference sampling, arithmetic, transfer, quadrature,
pressure gauge, region coverage, or spatial sufficiency. P09 and P10 remain
incomplete and accepted concentrating PDE windows remain zero.
