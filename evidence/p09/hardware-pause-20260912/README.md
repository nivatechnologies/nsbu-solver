# NSBU Solver hardware pause — 12 September 2026

User requested a hardware-change pause at 06:55 UTC. The previously authorized work window ended at 09:49 UTC, but this explicit pause supersedes it. Do not automatically restart numerical jobs after reboot; wait for the user to resume.

## Public and combined state

- Main: `4c996b8ab902000d345b8125847e979dc86fc926`, clean, pushed.
- Published prerelease: `alpha-20260912`, exact source `def4730b08025fdd06e7a8a0d78116aea24b6e2c`, downloaded binary and both source-matched hosted suites verified.
- Combined development checkout: `/mnt/niva-array/nsbu-solver/work/p09-continuation-20260912`, branch `codex/p09-continuation-20260912`.
- Zero accepted concentrating PDE windows; P08/P09/P10 remain incomplete.
- The combined branch includes accepted nominal coverage integration source but its final focused coverage/CRAP is incomplete; never promote the branch solely because it was backed up.

## Resume priority

1. Inspect local and remote branch state, review new hosted CI outcomes and preserve exact-source reports. Do not force-push or erase failed evidence.
2. Finish nominal-coverage final tests and CRAP from source `3a6306748bf0a1af7e6376684e48180d6603b71c`, worktree `work/p09-accepted-coverage-integration`. The fresh unique LLVM run completed regions 7/7, coordinator 3/3 and zero-allocation audit, then was user-interrupted during export (exit 130). Export, final JSON and CRAP remain incomplete. Earlier overlapping profiles are invalid and must never be merged as evidence. Partial evidence is `evidence/p09/v2-accepted-coverage-integration-pause/`.
3. Review and finish shared-force adapter WIP, branch `codex/p10-shared-force-adapter`, source `1efaa79a55fcfb176dc2f74126c92e2275a7b81f`, backup tip `6b435187090e7cad35c63b6dfd6c8fa779b09d5c`. Read its `evidence/p10/v2-shared-force-adapter-wip/` for exact dependency order, pre-split evidence boundaries and pending final-source gates. The adapter is NOT merged into combined development. Its interrupted coverage exited 143.
4. Review standalone offstage regional tracking WIP, branch `codex/p09-probe-regional-tracking`, commit `c5ffa68a8a74d05785a2b1835573a316bad0d027`. It is NOT merged. Fix the incorrect `SampledError` import (`nsbu_solver::diagnostics::local::SampledError`), then review resource/identity/transactional contracts before tests. No tests or quality gates have passed for this WIP.
5. Use the preserved N24/N32/N48 M96 midpoint comparisons to select the next bounded spatial experiment. Hardware-paused runs are partial, not numerical failures. Coefficient snapshots are NOT resumable cached-runtime archives; any new endpoint run must start independently from rest. Never inject an analytical reference or treat a coefficient file as a verified restart lineage.
6. CI acceleration remains a separate unimplemented prototype proposal at `work/p10-coverage-shard-prototype/work/coverage-shard-prototype/HANDOFF.md`. No tests or workflow edits were made. Check representative serial/parallel instrumented target equivalence before proposing a full-scope change; do not weaken coverage or CRAP gates.

## Preserved local state

All pre-existing dirty historical worktrees were copied non-destructively to `/mnt/niva-array/nsbu-solver/.git/hardware-pause-20260912/` with patches, file copies and checksums. Originals remain unchanged. The inventory captured 98 worktrees, 8 dirty, 90 changed/untracked nonignored file copies, 405505 bytes. Eleven old pressure-coverage watcher groups were explicitly stopped after PID/argv/cwd verification.

All three runs saved verified clock-2048 snapshots (128 attempts and 128 commits each) and were then terminated for the hardware pause. N24 last logged clock2224/139 commits; N32 and N48 last logged clock2064/129 commits. No endpoint completed. The last numerical process stopped at 07:09 UTC. See `evidence/p10/m96-user-interruption/summary.json` and `evidence/p10/m96-spatial-midpoint-user-pause-def4730/summary.json`. The two adjacent midpoint H1 differences are 8.79870862 and 13.02002519: this ladder does not demonstrate spatial convergence.

The historical source archive is now `11e9ffb711ab3644b8c5cb1cf02ca86936d8b258`, with the release tree unchanged. All three WIP branches are backed up remotely. Final continuation handoff and evidence are committed and pushed after this file is written. All local solver, build, test and coverage watcher jobs are stopped. Unrelated user shells, Claude and Obsidian processes were left untouched.
