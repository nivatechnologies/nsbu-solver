# Offline observer: real N384 parity verification

The frozen observer at `9adc021ae3e0d3bed3411f209f148d9f065f22ad`
reproduced all 11 archived inline balance values **bit-for-bit** on the actual
N384/M512 r6 state at tick 4096, with target 8192. The offline diagnostic and
force grids were 768, with 32 force workers. The run took 330.936 seconds and
peaked at 97,046,528 KiB RSS. Both whole-file and coefficient hashes were
reverified by the decoder. The archived trajectory was never changed or resumed.

[Result](result.json), [source and launch bindings](launch.json),
[output](output.json), [resource preflight](r6-final-preflight.json), and
[summary](summary.json) preserve the measurements. Absolute artifact paths in
these receipts describe the execution host; the large snapshots are not Git
contents. The copied frozen plan remains byte-identical. A separate eight-node
inventory checks headers/trailers only; it is not full payload verification.

The combined source passed 12 independent release-mode tests (26.53 seconds),
including the N64 AVX control, case mismatch, forged ledger, cap boundaries,
provenance and clock checks. Clippy with warnings denied passed. The repository
checker passed its bootstrap-only scope. New Rust source/test files remain
below 500 lines. No new whole-project coverage/complexity gate is claimed.

Compressed OpenCode transcripts retain the local workers' work and corrections.
The initial 27B case-review timeout produced no accepted result; continuing the
same session with the specific remaining patch succeeded. Root integrated its
case check, split tests by concern and independently tested the combined source.

This validates offline conservative-balance reuse at N384. N512 has only an
arithmetic resource estimate here (235,548,209,862 bytes); no N512 observer has
yet run in this packet. This is neither full fine-observable validation nor an
accepted PDE window. Accepted PDE windows remain zero.
