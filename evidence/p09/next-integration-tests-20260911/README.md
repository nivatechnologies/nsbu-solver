# P09 next-integration focused regression

This evidence binds a bounded normal locked regression to source
`e666994a22cd8fd5c6e1985c1858c773060653de`, after integrating the exact-v2
partial review adapter and diagnostic exporter. The starting and ending Git
HEAD were identical. All 15 hashed source, test, manifest and lock files were
reverified after execution.

The `v2_review_adapter` harness completed with exit status zero and emitted 28
partial-unqualified records. It measured 1:40.35 wall seconds and 766,504 KiB
maximum RSS. The `v2_diagnostic_export` integration harness passed all three
tests in 1:34.39 wall seconds with 292,344 KiB maximum RSS. The filtered
`diagnostic_export` library run passed three tests, filtered 40 unrelated tests,
and measured 0:04.21 wall seconds with 819,408 KiB maximum RSS.

These maximum-RSS values describe the complete observed test processes and can
include compilation. They are not adapter/export allocation reservations,
aggregate memory caps, or steady-state allocation claims.

No coverage command, full workspace suite, numerical threshold change, source
edit, or acceptance claim belongs to this regression. Complete stdout/stderr,
GNU `time -v` output and numeric exit status for every command are retained as
deterministically compressed raw artifacts. `commands.txt` gives the exact
commands. The preflight, postflight and per-file source verification are also
retained. Every run reached a terminal exit status of zero; none was aborted.
