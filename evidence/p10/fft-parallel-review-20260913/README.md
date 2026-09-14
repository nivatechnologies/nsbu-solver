# Parallel FFT final verification evidence

This packet verifies source revision `8161661972edb81c9a066900b43334997bb59b74` in the review worktree. The worktree also contained root-owned documentation/status edits; this verification did not alter source, documentation, or status files.

## Final checks

- `cargo fmt --all -- --check`: status 0 (`fmt.*`).
- Post-format provider coverage rerun: `RUSTC_BOOTSTRAP=1 CARGO_TARGET_DIR=/tmp/nsbu-root-parallel-coverage CARGO_BUILD_JOBS=2 cargo llvm-cov --no-clean --branch --json --output-path evidence/p10/fft-parallel-review-20260913/final-provider-coverage.json test -p nsbu-benchmarks --lib provider::parallel_reduced::tests -- --test-threads=1`; status 0, 4 passed, 0 failed. The retained target was not cleaned.
- Previously completed post-format solver coverage (`format-solver-coverage.*`): status 0, 265 passed, 0 failed. Filtering its LLVM file summaries to `crates/nsbu-solver` gives 16,766/17,162 lines (97.6926%), 30,350/31,465 regions (96.4564%), 1,428/1,459 functions (97.8753%), and 1,139/1,271 branches (89.6145%). Each solver-only measure exceeds 80%.
- The changed-scope RCA inputs were regenerated from the 17 solver files and 5 benchmark files already enumerated by `final-{solver,benchmark}-changed-metrics.json`. The branch-outcome union now names `format-solver-coverage.json`, `final-provider-coverage.json`, and `actual-harness-dependencies-coverage.json` as inputs.
- Changed solver scope: maximum function cyclomatic 15, cognitive 15, file Halstead difficulty 73.2131, and CRAP 23.625 (217 function/closure rows).
- Changed benchmark scope: maximum function cyclomatic 11, cognitive 11, file Halstead difficulty 58.1133, and CRAP 16.0 (88 function/closure rows).
- `python3 tools/check_repository.py`: status recorded in `repository.status`; its output is `repository.stdout`.

The earlier broad coverage attempt remains honestly recorded as aborted (`coverage.status` 143); it was not rerun. The repository-wide `metrics.json` remains preserved and still records the preexisting file-level Halstead difficulty 154.0769 in `crates/nsbu-solver/tests/fixtures/local_modal_derivatives.rs`. That fixture is outside this changed scope; this packet neither hides nor repairs that failure.

Two malformed intermediate provider rerun attempts are summarized in `provider-rerun-intermediate-failures.txt`. The successful final artifacts are `final-provider-coverage.*`. The first used stable branch instrumentation without `RUSTC_BOOTSTRAP`; the second mixed nightly profile version 11 with retained stable version 10 profiles. The single profile produced by that owned nightly attempt was removed before the final stable-toolchain, no-clean rerun. No preexisting profiles or compiled target files were cleaned.

Large JSON files are stored as deterministic gzip (`gzip -n -9`) to keep the packet bounded. `RAW_JSON_SHA256SUMS` preserves each original uncompressed SHA-256; decompression reproduces those bytes exactly. `SHA256SUMS` inventories every present packet file except itself, including the compressed forms and raw-hash manifest.

Repository recheck after transparent removal of unrelated host process arguments passed (see `repository-after-redaction.json`). Numerical results and frozen review files were unchanged; the process-log redaction record retains original and published hashes.
