# Exact-v2 pressure consumer evidence

This bounded increment constructs global mean-zero pressure and its complete
three-component gradient from all six actual exact-v2 family branches. It reports
the two spatial, two temporal and one CM/HO pair at each accepted family clock.
The diagnostic evaluates a fresh original `V2Force` on the complete doubled
finest grid and never receives an analytical pressure or integrator stage RHS.

The focused non-instrumented checks passed seven unique tests/probes: one private
full-force Poisson control, two independent convolution/DFT oracle tests, three
family admission/transaction tests and one allocator executable. The actual
family test compares RMS and peak values for pressure and gradient across all five
pairs. It also verifies report identity and clocks, unchanged state words, stale,
foreign, terminated and exhausted refusals, complete charging and schedule
progression only after a complete report. Construction allocated 23,020,352 bytes
within the 28,007,168-byte joint reservation; planning and steady measurement
allocated nothing.

Strict formatting, Clippy and Rustdoc pass. Fresh whole-source maxima are CC 21,
cognitive complexity 21, Halstead difficulty 75.8955 and 477 physical lines,
within the active limits. A focused LLVM coverage run was stopped during the
instrumented allocator executable after all 32 benchmark library tests passed;
it did not emit a final coverage report. Current-source line/branch coverage and
CRAP therefore remain pending rather than inferred from earlier evidence.

At elapsed tick 128, the two spatial pressure RMS differences are approximately
`2.9033e-13` and `1.1764e-13`; the corresponding gradient RMS differences are
`4.9015e-12` and `3.5812e-12`. The CM/HO difference is below the binary64 scale
of the common force pressure. Same-clock pair differences cancel the common
prescribed-force contribution and cannot qualify force sampling independently.
The private force-only control checks its sign, doubled-band modes and zero mode.

These are global sampled diagnostics at the startup endpoint `1/8192`. Complete
reference-gauge, regional, force-sampling, arithmetic and accepted-artifact
qualification remains pending. P09 and P10 remain open, and no concentrating PDE
window is accepted. Broader source-matched and hosted gates are pending.
