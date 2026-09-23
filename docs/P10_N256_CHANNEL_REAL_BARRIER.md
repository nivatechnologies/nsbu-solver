# P10 N256 scheduled-endpoint harness first-step channel admission repair

`evidence/p10/avx-scheduled-endpoint/harness` now carries the independently
reviewed source-level repair of the opt-in inherited decision-channel barrier:
the channel can no longer admit, arm, or finish outside its exact reviewed
first-step contract. This is a source-level preparation integration only. It
performs no solver execution, staging, launch or qualification, and it grants
no operational authority of any kind.

## What the repair closes

- Admission: channel mode is accepted only in the `n256-m512-piecewise-cadv33`
  profile and only with the armed clock equal to the profile's exact first
  scheduled clock 64; any other clock refuses with
  `barrier_channel_clock_unreviewed` before attempt 1.
- Arming: the channel disarms with `barrier_channel_first_step_mismatch` if
  the armed clock is ever reached at an attempt other than 1.
- Run end: a channel run whose barrier never armed or never resolved refuses
  instead of publishing the endpoint marker.
- Profile scope: the generic offline-capture family and the plain `test` cfg
  no longer admit the channel, so default-profile test success cannot
  masquerade as production-profile acceptance.

## Provenance

The integrated harness files are byte-identical to the reviewed package:
`0001-channel-real-barrier3.patch` (SHA-256
`a0b90d3f29ad8089a4f8febea5ea200270e37ea85694e9cb6b41dd6137d2a885`, sealed by
the donor package manifest SHA-256
`98deed10d8ec98a0123cda564a87072ab4d06a2eb19e56e59c3bcf3921e1e1bb`) was applied
to a private copy of HEAD `5bae94fa1258106771fb6025d96101c61e4f0cb5` with
`patch -p1 --fuzz=0`, zero offsets, and reproduced every harness file
byte-for-byte. The scoped independent review report has SHA-256
`7e196ff80340bfbcb6fe98bef85360a3ab32c704a8f2ca26ac158f70b3a8e959`, sealed by
its review-directory SHA-256SUMS root
`d3f39975c84b5f1a6fb3b4fccc7a60381b6d5ba47782730b96a4d93eb132a192`. That
review is a scoped source PASS only; it explicitly grants no transport,
custody, operational, or numerical qualification. No reviewed Rust source
byte was altered during integration; only this page and the
`p10_n256_channel_real_barrier_integration` status entry were added.

## Offline verification at this base

Fresh `--offline --locked` runs with `RUN_SOURCE` stamped to base HEAD:
default-profile harness tests 43/43 pass; N256 capture-profile tests 63/63
pass including the real local socket release/abort dispatch; N512
capture-profile tests 57/57 pass including cross-profile denial; strict
all-target N256 Clippy exits zero warnings; all six changed source files are
rustfmt-clean. The pre-existing default-profile dead-code Clippy baseline
warning carries forward unchanged as an open item.

## Six-file measurement state (partial; no quality-policy PASS)

Partial measurements exist for the six changed/created files under the N256
capture profile. Measured: LLVM region, line, and function coverage
(`cargo-llvm-cov` on the pinned stable toolchain) and rust-code-analysis
per-file cyclomatic and cognitive maxima plus Halstead volume/difficulty.
Measured line coverage ranges from 63.47% (`main.rs`, 58.48% region, 66.67%
function) to 100.00% (`barrier/tests_parser.rs`). Not measured: instrumented
branch coverage, because `cargo llvm-cov --branch` requires the nightly
toolchain while the repository pins stable; consequently per-function CRAP
is also not derived, because [the quality policy](QUALITY.md) computes CRAP
per function from actual branch outcomes as
`CC^2 x (1 - branch_coverage)^3 + CC` and no per-function branch outcomes are
retained. Region, line, and function coverage are not branch coverage, and
the rust-code-analysis cognitive figure is that tool's metric, not asserted
equal to the policy tool's. These partial data cannot establish the policy's
80% executable-line or branch gates for the modified scope, so no source
quality gate is claimed as passing here, and full quality-policy acceptance
remains unapproved.

## Subsequent complexity refactor

The channel parser and decision consumer now extract their existing checks into
private helpers. Independent source review found the same refusal order, ACK
behavior, deadline checks and one-shot decision behavior. With the pinned
rust-code-analysis tool, `env.rs::parse` changed from CC 23 to 11 and
`channel.rs::consume_with` from CC 22 to 14; the full harness source maximum
is 19. Fresh offline default/N256/N512 harness suites pass 46/66/60 tests,
and strict all-target Clippy passes in all three profiles. This is a
source-quality improvement only. The earlier six-file measurements above
describe the original admission-repair base; branch coverage and CRAP for the
current combined source still require a separately reviewed measurement before
the quality gate can pass. Transport, host authority, launch and PDE
qualification remain unapproved.

## What remains unapproved

Socket transport semantics, ACK delivery and supervisor ACK validation,
exclusive descriptor custody, durable decision custody, crash/restart and
post-send uncertainty handling, protected-host authority, full source
quality gates (including branch coverage and per-function CRAP), the
canonical binary, staging, launch, run lifecycle, and any actual N256
trajectory remain unapproved. The first-attempt launch admission gate remains
FAIL, `accepted_windows = 0`, and zero PDE windows are accepted. See the
[concentrating-window dependency notes](NEXT_CONCENTRATING_WINDOW.md) and the
[scheduled-endpoint harness evidence](../evidence/p10/avx-scheduled-endpoint/README.md).
