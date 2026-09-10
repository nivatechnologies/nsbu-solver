# P09 authenticated artifacts and raw-history replay

This increment supplies checkpoint components, not a complete checkpoint or a
completed P09. Accepted concentrating PDE windows remain zero.

`checkpoint::artifacts::Artifact` bounds nonempty input before verifying SHA-256
against its expected identity, preserving every original byte. `Catalog` requires
six canonical artifact slots and permits one optional reference slot under a total
byte allowance. Hash integrity does not validate an artifact's schema, scientific
adequacy or truth. It cannot prove that a claimed trajectory was integrated.

`AttemptRecord` now retains each accepted state's actual balance sample as well
as outcome and local indicators. `RunHistory::replay` bounds records and storage,
validates exact starting clocks and terminal outcomes, then reconstructs controller
and compensated balance history in their original operation order. Rejected or
refused attempts cannot supply accepted samples. Terminal histories cannot append
another attempt. An exact-cap valid log is admitted. Raw replay is an internal
consistency check; separately supplied physical state still needs coherent binding.

Both actual CM and HO runs continue after replacing their live history with its
replayed copy. A separate cancellation fixture retains a unit balance contribution
between large opposite-signed contributions; pending Simpson pairs remain explicit.
Known-answer hash, changed-byte, missing-slot, order, count and allowance controls
pass. A focused mutation uncovered the exact record-cap boundary test, which was
added before the complete verification run.

All 226 tests/probes pass across 172 maintained Rust files. All 11,879 executable
lines, 956 instrumented branches and 944 functions are covered. The completed
2,374-mutant run caught 2,186 and found 188 unviable, with zero survivors/timeouts.
These are actual results under the previous strict policy, preserved unchanged;
the subsequent user-authorized policy requires 80% coverage and informational
mutation/dead-code findings. They are not new zero-finding release requirements.

Maximum CC21, cognitive16, function/file Halstead74.4231, physical-file336 and
CRAP21 were measured. Strict linting and clone detection passed. Scoped SOLID/API
review keeps immutable content validation, history replay, observation and physical
state mutation separate; no private Niva or reference-state assignment is introduced.
Raw LLVM region/instantiation figures remain separately visible in the report.

Fresh public source packaging, repository checks, 41 bootstrap tests and the
original mathematical verification pass. The dependency and license-file inventories
record the new SHA-256 dependency graph. Original LICENSE, NOTICE and reviewed
inputs are unchanged. Test-only stats_alloc's missing separate registry license
file remains an explicit inventory limitation. The tested workflow hash belongs
to the earlier policy at commit 0830477; revised-policy hosted checks are separate.

Full coherent state/provider/reconstruction/lineage assembly, artifact schemas,
transfer and mandatory refinement studies, numerical CLI and concentrating
qualification remain incomplete. Artifact and raw-history binary codecs and the
physical transfer comparison are separate drafts, absent from this source snapshot.
