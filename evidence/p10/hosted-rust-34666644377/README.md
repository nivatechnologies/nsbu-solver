# Hosted Rust quality artifact

- Workflow: [34666644377](https://github.com/nivatechnologies/nsbu-solver/actions/runs/34666644377)
- Source revision: `56755bd0da070ceaa91e6980e2285c5a3d812b1f` (matches workflow head and local Git)
- Job: `rust`; conclusion `success`; completed `2026-09-12T07:00:30Z`.

Aggregated LLVM coverage (summing recorded file summaries):
- lines: 45832/46995 (97.525269%)
- branches: 2900/3330 (87.087087%)
- functions: 4039/4123 (97.962649%)
- regions: 84104/88579 (94.948013%)

- Rust CRAP maximum: `24.05859375`
- Top CRAP functions:
  - `crates/nsbu-benchmarks/src/v2_run/work.rs::validate_outcome`: 24.05859375
  - `crates/nsbu-benchmarks/src/v2_experiment/coverage/plan.rs::new`: 23.93586006
  - `crates/nsbu-benchmarks/src/v2_run/archive.rs::read_reservation`: 22.50000000
  - `crates/nsbu-benchmarks/src/provider/parallel/pool.rs::execute`: 22.50000000
  - `crates/nsbu-benchmarks/src/v2_run/archive/admission.rs::admit_frame`: 21.91210938

All recorded build/lint/doc/package/coverage/CRAP/source-report steps concluded successfully; the informational mutation step was skipped. This archive contains quality reports only and makes no PDE qualification or package-completion claim.

Raw report files are deterministic gzip members. `ORIGINAL-RAW-SHA256SUMS` records hashes of the downloaded uncompressed artifact files; `SHA256SUMS` records archive-local compressed files. All paths are relative to this evidence directory.
