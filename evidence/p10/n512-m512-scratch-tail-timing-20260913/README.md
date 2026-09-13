# N512/M512 scratch-tail one-attempt timing preparation

This directory prepares, but does not launch, one actual Cox--Matthews attempt
from exact rest over clocks 0 through 64. It retains N512 and samples the force
at M512, matching the spatial anchor recorded by `ac6739001bbf9531a4074dd23fd38c64f7abc040`.
M384 is not an admitted force layout for this retained state. The numerical
method, tolerances, advective limit, clock, and sole-attempt rule are identical
to the completed N512/M768 timing diagnostic.

The production implementation is the AVX scratch-tail source
`0843b8b18e6a096a0208e3d896e391c7b1b2f5e0` with the descendant test-only
reservation correction `9eba11f196a25f0843f0cbd0f4ed08c9f7ae4645`.
This harness is a narrow adaptation of `5f71c97cbc62bf176901732e57c6adefbf1861a5`.
The historical N512/M768 harness and run evidence remain unchanged.

The exact API resource preflight closes at 207,576,840,688 bytes. Force uses
W3 on M512, while the nonlinear RHS continues to use bidirectional W3 on the
N512 padded M768 layout. The external address-space cap remains 256 GiB and
the launcher retains the conservative 272 GiB MemAvailable floor. Focused
serial tests cover the exact resource classes, each type-specific one-byte-under
refusal before allocation, exact rest-to-64 clock stages, execution gating, and
the canonical candidate-hash byte fixture.

The candidate hash is harness-only instrumentation after the timed attempt.
An accepted token is borrowed through the public `proposal` API and read without
mutation. SHA-256 consumes components in axis order, coefficients in stored
order, and each real then imaginary binary64 word in little-endian order. The
token is dropped; no candidate is committed or published.

A read-only Sulaco snapshot at 2026-09-13T18:14:49Z reported 257,334,657,024 bytes available. The exact internal peak fits with 49,757,816,336 bytes remaining. A possible host-specific gate of exact peak plus the existing 16 GiB operational margin would be 224,756,709,872 bytes and also fits that snapshot, but it is only a proposal for separate review. The prepared 272 GiB floor is unchanged and therefore does not currently admit Sulaco.

The prepared launcher allows 2700 seconds plus a 60-second kill grace and keeps
300 seconds for review and cleanup before the campaign deadline. The latest
launch is 2026-09-13T19:01:54Z. No numerical attempt has been executed here.
Any future local acceptance is an integrator outcome only, with no PDE
qualification or accepted-window claim.
