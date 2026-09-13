# N512/M768 scratch-tail one-attempt timing preparation

This directory prepares, but does not launch, one actual Cox--Matthews attempt
from exact rest over clocks 0 through 64. It uses the final AVX scratch-tail
production source `0843b8b18e6a096a0208e3d896e391c7b1b2f5e0`, the descendant
test-only correction `9eba11f196a25f0843f0cbd0f4ed08c9f7ae4645`, and a narrowly
adapted copy of the existing `6442ec98` one-attempt harness. The prior harness
and all prior run evidence remain unchanged.
The inherited `cache.rs` and `timed_rhs.rs` helpers have no transitive path
modules and their hashes exactly match the copies at `6442ec98`.

The allocation-free reservation preflight closes at 238,210,324,976 bytes,
589,824 bytes above the serial baseline. The external address-space cap remains
256 GiB and launch requires `MemAvailable >= 292,057,776,128` bytes. Six focused
tests pass serially, including the one-byte-under constructor refusals and the
canonical candidate-hash byte fixture. Clippy with warnings denied and the
locked release build pass. `prepared/source-diffs/` preserves the complete
production, test-only, and harness adaptation diffs; `prepared/source-sha256.list`
binds the candidate sources.

The candidate hash is narrow harness instrumentation. After the timed attempt,
an accepted token is borrowed through the existing public `proposal` API and
the three components are read without mutation. SHA-256 consumes components in
axis order, coefficients in stored order, and each real then imaginary binary64
word in little-endian order. The token is then dropped. No numerical API was
changed, and no integrated state is committed, written, or published.

`serial-baseline.json` binds equivalent numerical inputs from the prior
2209.71-second wall-clock run. That result did not record a candidate
coefficient hash, so it supports timing comparison only and cannot support a
whole-attempt bitwise comparison.

The guarded launcher allows 2700 seconds plus a 60-second kill grace and keeps
300 seconds for post-run review and cleanup before the absolute campaign
deadline. Its latest admissible launch is 2026-09-13T19:01:54Z. It requires an
explicit root launch variable, explicit W3 resource-release confirmation, no
live W3 control command, exact sources and binary, sufficient memory, a fresh
output directory, at least 1 GiB free disk, and a twice-stable
PID/PGID/starttime/cmdline identity.

No numerical attempt has been executed here. A future local accepted token
would remain an integrator outcome only, not PDE qualification or an accepted
window.
