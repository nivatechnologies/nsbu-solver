# P01 workspace verification

The workspace builds on stable Rust 1.94.0. Three process integration tests cover
help, version, empty arguments, unsupported commands and extra arguments. All
three package archives pass Cargo's package verification build. A local
`cargo install --path crates/nsbu-cli --locked` installation executes successfully.

| Maintained Rust metric | Measured result |
|---|---:|
| Cyclomatic complexity, maximum function | 9 |
| Cognitive complexity, maximum function | 2 |
| Halstead difficulty, maximum file/function | 15.5 / 11 |
| Physical lines, maximum source/test file | 49 |
| Executable line coverage, including tests | 53/53 |
| Instrumented branch coverage | 8/8 |
| CRAP maximum, at full branch coverage | 9 |
| Mutants generated / caught / unviable | 13 / 12 / 1 |
| Missed / timeout / untested mutants | 0 / 0 / 0 |
| Detected duplicated fragments | 0 |
| Confirmed dead / redundant code after review | 0 / 0 |
| Dynamic Any / unresolved types | 0 / 0 |

`coverage.json` includes integration-test code. Const-only library and doc-only
benchmark files contain no executable lines. LLVM reports 6/6 functions and
6/8 instantiations: duplicate test-harness function instances are not invoked.
Line/branch gates do not claim complete instantiation coverage or MC/DC.
`metrics.json` contains concatenated per-file JSON objects (`jq -s` reads them).
Raw numerical tool results are preserved; machine-specific checkout/toolchain
path prefixes in published reports are replaced by `<checkout>` and `<rustup>`.
Local unsanitized outputs remain under ignored `work/`.

Mutation's one unviable replacement returns an integer from `main -> ExitCode`;
its compile error is recorded separately, never as a caught mutant. The 12 caught
mutants each produce explicit failing tests, with zero process timeouts.

SOLID review: argument parsing is pure and separate from process I/O; the CLI
uses the library facade and the library does not depend on CLI or benchmark
code. The benchmark crate is explicitly reserved. Both public metadata constants
are used by the CLI. Every private function is exercised. No numerical API or
speculative abstraction has been introduced. Strict compiler/Clippy warnings
cover private dead code; manual review covers exported constants and apparent
duplication. Five Rust files are included in metrics, including tests.

The FFT selection spike separately tests realfft 3.5.0 with rustfft 6.4.1,
default features disabled, on sizes 4, 6, 8, 12, 128 and 192. Round-trip error is
at most 1.776e-15 and scratch requirements are recorded. These public pure-Rust
libraries use compatible licenses and require no native FFT library. Their
workspace versions are pinned for P03; no FFT dependency is linked in P01.

This verifies package infrastructure only. No Rust numerical integration,
concentrating trajectory or accepted PDE window follows from these results.
See [summary.json](summary.json) for current completion state and source hashes.

The fresh public-only checkout build/test/install and hosted Rust quality job
passed at revision `7640e19b06a2cc0e01dbcbb2c9ab7c9453ad6dd8`. P01 is complete;
[hosted-ci.json](hosted-ci.json) records every successful workflow step.
