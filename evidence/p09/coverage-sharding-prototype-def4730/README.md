# Coverage execution sharding prototype

This is a bounded infrastructure prototype at source `def4730b08025fdd06e7a8a0d78116aea24b6e2c`, not a replacement for the hosted Rust gate. It inventories 201 Cargo executable candidates for `--workspace --all-targets`, then validates three representative Cargo test artifacts: a standard test harness, an explicit `harness = false` allocation binary, and an example artifact. Each artifact ran once with serial execution and once with at most four workers. Each process wrote an isolated LLVM profile.

The serial and parallel merged branch JSON files have the same SHA-256: `de039221a8c05bd28389641efc0d7f9319b7d81b88851a2eaabef643e01e195e`. They contain 2,506/13,799 covered/executable lines, 230/1,528 covered/instrumented branches, 268/1,528 covered/functions, 180 files and 1,641 function records. The runner compares the full totals, per-file summaries and top-level function records; the raw report byte identity additionally covers the branch records. These low coverage percentages describe only the deliberately small subset.

The prototype does not run the full workspace, calculate CRAP or static metrics, or include doctests. The existing hosted workflow keeps doctests separate. It makes no numerical, readiness, or accepted-window claim, and it does not alter a workflow. The two early parser/instrumentation failures are retained in `raw/initial-failure.md`; they were corrected before the final branch-enabled run.

The runner discovers `cargo-llvm-cov` on `PATH` by default. A local tool directory can be supplied with `--tool-bin` or `NSBU_RUST_TOOLS`; the runner validates that the directory exists and contains `cargo-llvm-cov` before prepending it to `PATH`. It contains no machine-specific tool location.

`artifact-sha256.json` intentionally excludes itself to avoid a recursive checksum. Its listed 12 artifacts were independently rehashed after this portability correction.

See [summary.json](summary.json), [commands.txt](commands.txt), the [target inventory](raw/all-target-inventory.json.gz), and compressed raw reports under `raw/`.

## Python quality follow-up

The maintained configured Python scope passed after the runner and seven focused orchestration tests were added: strict basedpyright reported zero errors and warnings; complexipy found every function at or below 21; full `pytest -q` passed under branch coverage; coverage measured 6,394/6,483 executable lines and 1,223/1,264 branch outcomes (98% displayed); the per-function CRAP maximum was 20. Compressed raw reports are retained in `raw/`, and [quality-followup.json](quality-followup.json) binds this result to the runner and test hashes.

This remains **representative-only** infrastructure evidence: it exercises three artifacts and does not demonstrate full 201-target artifact inventory parity, a hosted speedup, CI workflow integration, or a replacement for required Rust coverage, CRAP, static, or doctest gates.
