# P10 one-clock offline residual probe

This package prepares one bounded, read-only diagnostic at clock 1112 from the completed
N384/M384 Cox--Matthews piecewise trajectory. It does not deserialize a runtime owner, resume a
trajectory, publish an accepted state, qualify interpolation, or add an accepted window.

The three reconstructions use supports `[896,1152,1408]`, `[1024,1152,1280]`, and
`[1088,1152,1216]`. Their interval widths and spacings are nested; their node sets share only
clock 1152. The consumer hash-decodes the seven immutable snapshots against the frozen bindings
in `probe-plan.json`. It computes each node derivative freshly with the archived-equivalent W3
rotational RHS and the M384, 32-worker attempt-cache force profile. The production constructor is
the archived AVX catalog path: an N576 width-three bidirectional W3 rotational transform and an
M384 width-three forward W3 cached parallel-reduced-force transform. A small constructor-selection
control compares this W3 path bit-for-bit with the archived scalar AVX reference path. Because
that explicit RHS omits the ETD linear term, the consumer adds `-nu |k|^2 u` exactly once before
Hermite reconstruction.

For each reconstruction, the residual path independently samples the force on M768 at clock
1112 with the scalar AVX parallel-reduced provider, evaluates the conservative product with the
shared AVX catalog on the complete doubled N768 grid, and calls the existing residual kernel.
The report contains residual-acceleration L2, H1, curl L2, and divergence L2 norms, hashes each
full residual field, and reports coarse-to-middle and middle-to-fine residual-field and
value/derivative differences. It
makes no velocity-budget or pass claim because no reviewed dimensional comparison policy has
been supplied.

Storage is sequential. At most three node values and three node derivatives are resident. The
shared center is evaluated once; the two side nodes are evaluated per support. The current and
previous reconstructions and the previous full residual are retained only for adjacent
differences; the previous residual is included in both the reconstruction and residual phase
admissions. The checked conservative
admission peak is 117,217,453,000 bytes. The scoped 128 GiB cap leaves 20,221,500,472 bytes of
headroom above the checked owned classes. A live-run cap must cover this reservation after other
active reservations are subtracted from `MemAvailable`.

`cargo test` includes frozen-lineage and command-shape refusals, confined snapshot paths, a
linear Hermite reproduction, and a manufactured shear-mode sign control. The sign control
confirms that the explicit RHS contains no viscous decay, adding `-nu |k|^2 u` produces the
physical derivative, and the residual kernel adds the opposite viscous term once.

All numerical output is binary64 empirical data. It has neither outward rounding nor interval
enclosure and is unsuitable for a rigorous bound without a separate reviewed arithmetic layer.

## Fine residual localization extension

The reviewed extension executes only the fine support `[1088,1152,1216]` at clock 1112. It
splits every non-N768-Nyquist residual mode with the strict `Layout::locate` N384 mask, reports
per-term norms and signed cancellation data on the retained band and new shell, and preserves the
completed base-residual coefficient operation order as a SHA-256 replay gate.

A discrete retained-force control samples the exact integration `CachedReducedForce` constructor
into N384/M384 storage and applies strict zero padding. During the existing N768 traversal it uses
`R384=R768+P(f768-pad(f384))`; the plus sign follows from the `-P f768` already present in the
conservative term. This changes the discrete target equation outside the retained band and is
reported only as a force-resolution diagnostic. It performs no second conservative FFT set.

The fine-only checked reconstruction peak is 49,781,517,672 bytes. Including both conservatively
admitted provider classes and the retained-force field, the residual/control peak is
114,575,696,272 bytes. The unchanged scoped 128 GiB cap leaves 22,863,257,200 bytes before a live
host admission check. The detailed algebra, work accounting, controls, and claim limits are frozen
in `localization-proposal.json` and `localization-proposal.md`.
