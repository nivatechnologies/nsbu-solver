# N512 early-clock offline balance (first actual N512 observation)

The first **actual N512 run** of the frozen offline captured-observer
(`9adc021ae3e0d3bed3411f209f148d9f065f22ad`, binary
`66b4e2f4105db5377a207257e272f75ed0763e49f0493d50333cfc3be6a7cb8b`)
succeeded on the durable from-rest N512/M512 capture at **clock 1536**
(target 8192, exponent -20, epoch 24, 24 accepted steps). N512 is retained;
the diagnostic and force grids are the doubled **1024**; **32 workers**;
conservative ledger upper bound 235,548,209,862 bytes against a 300,647,710,720
byte cap.

The run exited 0 in **815.3000466823578 seconds** wall. Peak RSS is
**229,929,984 KiB**, taken directly from the actual `time.txt`
(`Maximum resident set size`), not estimated. **All 11 balances are finite**
(values in [result](result.json) and [summary](summary.json)). The actual
capture clock, the coefficient hash and the whole-file hash were all
reverified by the decoder against the copied record and receipt.

Scope is **balance-only**: the manifest kind
`FORCE_RESOLUTION_DIAGNOSTIC` selects decoder CM512 admission only; this run
makes no force-comparison claim and no convergence claim. The trajectory
endpoint 4096 stays distinct from the observed clock 1536; the immutable
from-rest capture was neither resumed nor reset (`resumable=false`). No
inline N512 control exists; the earlier N384 all-11-bit parity in the
frozen historical [verification-20260914 packet](../offline-captured-observer/verification-20260914/README.md)
supports observer reuse but does not prove numerical convergence here.
**Accepted PDE windows remain zero**; this is partial offline balance, not
full fine-observable, pressure or regional acceptance.

The 3,233,809,650-byte `state.bin` was **not** copied into this packet; it
remains at `work/n512-early-observer-artifacts-20260914/clock1536/state.bin`
with SHA-256 `7a4f05338c3cb7308a18c3788f2cc3a192fea9a93e3c1a07980b9963a3dbff55`,
matching the [copy receipt](clock1536-copy-receipt.json), the
[manifest](clock1536-balance-input.json) and the output's reverified hash.
Only the small metadata listed in [SHA256SUMS](SHA256SUMS) was copied,
byte-for-byte; the [record](clock1536-record.json) is byte-identical to the
harness copy `evidence/p10/offline-captured-observer/harness/clock1536-record.json`.
