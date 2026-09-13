# Ordered-Hessian screen review handoff

This handoff binds the existing immutable N192, N256, and N384 piecewise Cox-Matthews/M384
spectra at clocks 512 and 4096 for the optional ordered-Hessian screen reviewed at source commit
`861b25d`. It proposes two coarse/fine screens at each clock: N192->N256 with exact cap
578,486,272 bytes, then N256->N384 with exact cap 1,772,879,872 bytes. Clock 512 must run and be
archived before clock 4096.

The numerical kernel is FFT-free and allocation-free beyond the adapter's two already admitted
snapshot vectors. It reports the full-fine-band ordered Hessian difference and fine absolute scale
as physical-domain L2 and volume-average RMS, plus their ratios. All manifests preserve exact
state, plan, identity, profile, guard, source, backend, execution, schedule, clock, and hashes.

These outputs have no assigned Hessian budget and make no acceptance, pointwise, time-supremum,
finest-grid, or continuum claim. No run is authorized by this handoff; root source/test and
semantic-manifest review are required first.
