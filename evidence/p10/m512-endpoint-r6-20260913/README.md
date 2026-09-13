# Fresh M512 endpoint r6 run evidence

This directory records the bounded fresh-from-rest Sulaco endpoint run launched
through the reviewed r6 wrapper. The run is experimental and does not imply an
accepted PDE window.

`ownership.json` binds the launched solver and watchdog identities captured by
the wrapper. `first-step.json` records the passed clock-64 admission gate,
`clock-0512.json` records the required comparison-only r5 prefix check, and the
NUMA files record read-only placement observations. The second NUMA observation
is explicitly post-observer because the monitor result arrived after the first
observer completed. Final exit and artifact evidence remain pending.

`checkpoint-20260913T115254Z.json` records the requested durable-progress
checkpoint; its live remote sample completed twelve seconds after the requested
wall-clock instant.

`checkpoint-20260913T135254Z.json` records the later requested checkpoint. Its
remote resource sample completed after the clock-1600 commit and 74 seconds
after the requested wall-clock instant.

`force-comparison-binding.json` identifies the actual independently evolved
N384/M384 counterpart for the future clock-3584 and clock-4096 r6 comparisons.
It excludes the analytical projection and records the missing reviewed adapter
binary as a post-run dependency.

`adapter-build-binding.json` records a fresh isolated local build from the exact
reviewed force-diagnostic source and its focused validation. It has not been
staged or used for a large-state comparison.
The adjacent `adapter-*.log` files preserve the raw merged output from the
reviewed incremental test, Clippy, and release-build commands.

`clock-2048.json` records the next exact r5 prefix hash match and its scheduled
observer/resource evidence.

`clock-3072.json` records the final exact r5 prefix match before r6 enters the
previously uncomputed endpoint segment.

`clock-3584.json` and the paired `force-clock3584-*.json` manifests bind the
first newly completed endpoint-segment node. The comparison remains unexecuted
until the solver exits and root reviews the exact manifests.

`clock-4096.json`, the paired `force-clock4096-*.json` manifests, and
`final-run.json` bind the completed endpoint, clean exit status, final artifact
hashes, and the remaining comparison review gate. The solver exited zero after
5:47:52, well before the absolute deadline. The adapter comparisons have not
been launched and no acceptance conclusion is claimed.

`force-comparison-results.json` and `force-comparison-run/` preserve the two
reviewed post-exit force-resolution diagnostics at. Both clock 3584 and 4096
completed with status zero. Their acceptance field remains `not_assessed` and
these diagnostics do not establish force-channel PDE qualification.

The executed `7cc732e...` launcher bound the precomputed exact-argv command
hash. Its capture loop checked for two stable samples but lacked a final
explicit counter assertion, leaving a theoretical final-iteration one-sample
edge. The actual launches completed normally with expected command hashes.
The separately prepared clock-2560 evidence closes that edge before any future
launch.

The independently evolved M384 versus r6 M512 force diagnostic at clock 2560
also completed with status zero, closing the eight nonzero observer-clock
inventory. `force-comparison-clock2560-run/` preserves its raw result and
execution receipts.

`durable-archive-receipt.json` binds a complete local archive outside the Git
worktree: all 48 state files and 98 metadata files (65,569,755,172 bytes) match
the remote SHA-256 inventory exactly. Large state files remain outside Git.
