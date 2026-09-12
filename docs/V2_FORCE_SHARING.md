# Exact-v2 original-force sharing prerequisite

The original exact-v2 force depends on its complete `TickClock`, the fixed unit-cube,
unit-viscosity physical case, and its force sample grid. It does not depend on the
evolving velocity. Focused regression tests show that evaluating on a larger retained
Fourier domain and applying the existing strict-band crop reproduces a direct evaluation
on the smaller retained domain bit for bit.

The exercised configurations are N4/N8 with M16 and the independently doubled observer
configuration N8/N16 with M32. Both serial and two-worker evaluators are checked at exact
rest and at nonzero tick 2047 of target 8192. The oracle independently maps positive and
negative transverse modes, retains DC, and zeros all target Nyquist entries before every
coefficient word is compared. The production transfer is separately checked against this
mapping.

This result is only a prerequisite for a future shared force owner. It does not implement
shared storage, change a trajectory, cover different force sample grids, or establish
force-grid sufficiency. Integration M and independently doubled observer 2M remain distinct
numerical settings. The exact-v2 provider currently refuses nonunit physical lengths or
viscosity, so this regression makes no claim for nonunit domains.

A future bounded owner must still bind the full clock and physical/sample/domain settings,
reserve its provider and table exactly once, declare every possible request clock, and fail
closed on missing or stale entries. The current fixed v2 run terminates on rejection; the
separate generic retry scheduler can request additional dyadic clocks and cannot be assumed
to fit the nominal quarter-stage lattice.
