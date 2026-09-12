# Exact-v2 sampled physical force-grid evidence

Source commit `50222d71b7a31030af00dac52557d605ba71cf2f` is based on
`31e99a1`. Three independently evolved N=4 trajectories use nested prescribed
force grids M=[4,8,16], a fixed CM 64-tick step and endpoint 512. One common
12-cubed physical lattice measures velocity, all nine ordered gradient entries,
all 27 ordered Hessian entries, and all three curl components. RMS divides by
1,728 sample points and never by component count.

At both nonrest clocks 256 and 512, the independent oracle reconstructs signed
full-complex Fourier sums at every lattice point. It compares RMS, absolute
peak, relative peak and finer-state reference peak for all four quantities and
both M pairs. It additionally reconstructs each reported peak witness at its
reported index. Symmetry-related nonrest maxima may select different
representatives under direct-sum and FFT roundoff; the reported node must still
attain the independent maximum within the stated binary64 allowance. At exact
rest all error/reference values tie at zero, and every report selects the first
linear node 0 with coordinate [0,0,0].

The first expanded-oracle run failed because it required identical nonrest
representative indices for a symmetry-related gradient maximum. That false tie
requirement was replaced by the reported-node reconstruction described above;
the numerical values and production implementation were unchanged.

Endpoint RMS/peak values are preserved in `actual-measurements.log`. They are
nonzero for every quantity and pair. This is sampled, current-grid binary64
sensitivity evidence with permanent `DiagnosticOnly` status. It does not infer
gradient or Hessian RMS from spectral H1, and it establishes no force precision,
sufficiency, monotonic convergence, continuum bound, pressure result, regional
result, or accepted PDE window.
