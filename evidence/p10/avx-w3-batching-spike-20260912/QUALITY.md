# Corrective packaging and quality evidence

Commit `e1d5a31` put the complete 869-line harness in `src/main.rs`. This
corrective change only separates that prototype into cohesive modules. It does
not change production code, add a benchmark, or alter the recorded numerical
or timing evidence.

The source line counts after the split are:

| file | lines |
|---|---:|
| `benchmark.rs` | 258 |
| `controls.rs` | 126 |
| `main.rs` | 22 |
| `model.rs` | 99 |
| `pool.rs` | 459 |
| `records.rs` | 91 |
| `util.rs` | 54 |

`cargo fmt --check`, the release focused test, and release all-target clippy
with `-D warnings` pass. The focused N=6 test exercises numerical controls,
serial/W3 equality, deterministic publication, injected error and panic
drainage, one-byte-short cap refusal, allocation accounting, benchmark record
assembly, and the direct DFT comparison. It does not repeat the large profile.

Rust-code-analysis found 93 function/closure records. Function maxima are
cyclomatic complexity 15 (`ParallelBatch::new`), cognitive complexity 6
(`direct_dft`), and Halstead difficulty 30 (`reservation_parts`). These clear
the required strict limits of 22, 22, and 80. LLVM coverage is 741/799 lines
(92.7409%), 71/78 functions (91.0256%), and 49/74 branches (66.2162%). The
repository `quality/check_crap.py` report has maximum per-function CRAP 20,
clearing the strict limit of 25. No coverage or analysis exclusion is used.

The evidence is retained in `rust-metrics.jsonl`, `coverage.json`, and
`crap.json`. The original large-run artifacts remain byte-identical:

- `profile.stdout`: `ca3165df39f14d5753c9dac567ec268fa7635f585a3d7f5e4425d6aa5b459f6f`
- `profile.time`: `90de050ade5e04bd4c8a42e6ca578fa4224c019836dcd3d6eb54f9fc059593b5`
