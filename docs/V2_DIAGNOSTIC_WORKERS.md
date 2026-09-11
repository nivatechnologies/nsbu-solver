# Exact-v2 diagnostic force workers

The exact-v2 pressure and off-stage residual consumers honor the worker count
already stored in their bound trajectory or probe-family settings. They use the
existing `RunForce` selector, which retains the original `V2Force` point
arithmetic and serial Fourier transforms while optionally distributing physical
point sampling over persistent workers.

The sampling profiles are unchanged. Pressure uses its common finest doubled
domain layout. All six residual children use the configured integration-force
layout doubled once, currently `M=24` for the focused fixture, while retaining
their own `2N=8/16/24` output bands. A worker count of zero selects the serial
provider. Positive counts select that exact number of persistent workers; the
effective layout and worker count are retained in every immutable result.

Admission uses the selected provider's complete declared storage. For parallel
sampling this includes provider buffers, worker metadata and configured thread
stacks before any worker is started. Focused allocator programs exercise worker
counts one and two, reject a joint cap one byte below admission, remain within
the admitted construction bytes and allocate nothing during repeated diagnostic
measurements.

The numerical controls evolve each fixture from rest and compare workers one
and two against a freshly constructed serial `V2Force` on the same accepted
pressure states or reconstructed residual fields. Force coefficients and the
resulting pressure or residual coefficients agree bit for bit. The residual
control measures the complete six-branch `M=24` call as an informational fixed
input timing; timing is not an acceptance threshold.

This performance option does not change equations, force grids, transforms,
conservative kernels, default settings, CLI or archive formats. It does not
qualify force coverage, residual convergence, pressure gauge convergence or an
accepted PDE window. P09 and P10 remain incomplete.
