# Five-file static metrics and CRAP

RCA 0.0.25 measured the five changed production FFT files at exact source `9eba11f196a25f0843f0cbd0f4ed08c9f7ae4645`. CRAP was computed by the repository `quality/check_crap.py` against the source-bound LLVM JSON archived in the parent directory. Path matching uses exact repository-relative RCA names against coverage filenames ending in those paths.

Maximum function cyclomatic complexity is 14, maximum function cognitive complexity is 15, maximum Halstead difficulty across RCA file/function/container nodes is 74.667, and maximum file SLOC is 258. These pass the declared `<22`, `<22`, `<80`, and `<500` thresholds.

CRAP does not pass: `avx.rs::from_parts` measures CC 11, branch coverage 0.5, and CRAP 26.125 against the `<25` threshold. The uncovered `Debug::fmt` functions are retained honestly and each scores CRAP 2.0. No tests, exclusions, production edits, dead-code gate, or mutation gate were added. `crap.json` preserves every function score and `function-metrics.json` preserves per-function static metrics joined to coverage.
