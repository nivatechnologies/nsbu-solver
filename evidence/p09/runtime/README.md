# Bounded smooth runtime and binary components

[The execution summary](summary.json) binds this increment to SHA-256 hashes of
all 198 maintained Rust source, test and example files. Local verification passed
263 harness tests and four isolated allocation executables. Coverage measures
15,910/16,039 executable lines (99.20%) and 1,182/1,244 instrumented branches
(95.02%). Maximum CC is 21, cognitive complexity 16, function/file Halstead
difficulty 75.8956, physical file length 400 and CRAP 21. Raw metrics, coverage
and per-function CRAP reports are retained as gzip files beside the logs.

The installed `nsbu smooth` command independently evolves the smooth CyclicSine
case from rest with CM or HO. Seven installed-binary checks cover help, version,
dry-run, both methods, insufficient memory and invalid syntax. The package test
uses a fresh Cargo package target; installation and warnings-denied Rustdoc pass.
The runnable library example is included in the all-targets coverage scope.

The owner retains physical state, exact clock, controller, raw balance history,
per-attempt provider work and observer work. Binary readers check version, size,
resource caps and internal consistency. Tests reject rehashed counter/epoch
forgeries and preserve signed-zero/physical bits. Imported runs remain
`ExternalUnverified`, including after internal snapshots. Byte integrity cannot
authenticate claimed provenance. Archive continuation and complete recorded
steps are covered by allocation probes. A real coarse-to-fine forced-shear
comparison retains the error inherited from missing coarse modes.

SOLID review keeps numerical ownership, read-only measurement, serialization,
resource admission and CLI reporting separate. Independent conservative products
remain separate from the integration RHS. Clone detection reports 17 matches
(141 lines, 0.6145%): repeated imports and fixture setup, explicit archive field
layouts, checked size arithmetic, and separate negative-control assertions.
These are retained for clarity and test independence. Clippy and public-API review
are the scoped dead-code checks; this increment makes no universal absence claim.
Mutation sweeps were not rerun; older source-specific reports remain historical
evidence under the revised informational mutation policy.

P08 and P09 remain incomplete. Reconstruction storage and lineage event codecs
are components, not complete accepted-state provenance. The smooth CLI does not
yet save or resume files. Full reconstruction checkpoints, experiment integration
and concentrating refinement studies remain. **Accepted concentrating PDE
windows: zero.** [Hosted Rust](hosted-rust.json) and [Python](hosted-python.json) verification
passed for source revision `95590297a36a3ccff71b8a9eac4958040a60c3a5`.
