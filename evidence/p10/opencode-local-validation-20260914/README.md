# Local OpenCode solver-validation work

Three parallel OpenCode jobs used the configured local models against source
`9bb4780510397bac45500a177d1592a14c61ffa5`. Flash Next produced seven Rust
regression tests for snapshot decoding and fixed-M512 comparison admission;
27B produced five Python monitor tests. Each used an isolated worktree and a
20-minute process timeout. The two-hour custom JSON runner was not used.

Root reviewed the patches, removed one redundant import and warning suppression,
and formatted the new Rust files. The combined adapter suite passed 31 tests;
the monitor suite passed 6 tests. Clippy passed with warnings denied. The
archive includes model tool events, prompts, final reports and independent
checks. Cargo refreshed the adapter lockfile for the already-integrated Rayon
dependency; numerical production source was unchanged.

These checks improve offline validation preparation and establish successful
local edit/test workflows. They do not establish general model parity,
sustained throughput, an implemented full offline observer, or an accepted PDE
window. Model-side repairs are visible in the transcripts; success was not
necessarily on the first compilation.
