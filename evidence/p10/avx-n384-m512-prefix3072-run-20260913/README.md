# N384/M512 bounded prefix run through clock 3072

This archive records the one authorized Sulaco trajectory launched from rest with
the frozen N384/M512 profile. It produced 40 durable accepted snapshots through
clock 3072. The first 32 steps used h64 and the next eight used h128. Scheduled
observers completed at clocks 512, 1024, 1536, 2048, 2560, and 3072.

The prefix controller remained disabled for the entire run. The manual clock-2048
gate admitted exactly one continuation segment. At durable
clock 3072, the frozen formula used the maximum integration time from attempts
33--40 and the maximum positive observer time seen in the run:

```
Imax = 361.490995328 s
Omax = 293.017052202 s
B = ceil(1.15 * (8 * Imax + 2 * Omax + 300)) = 4345 s
remaining at 2026-09-13T05:56:04Z = 3836 s
```

Clock 3072 had already published at `2026-09-13T05:52:27.860164714Z`. The
gate was evaluated afterward. Because `B > remaining` by 509 seconds, the parent
numerical orchestrator sent TERM to the exact owned process group. Attempt 41 had
begun in memory after publication, but it remained unpublished and its partial
work was discarded by the manual stop; no attempt-41 directory or record exists. All launcher,
timeout, solver, and watchdog PIDs were subsequently observed gone. The watchdog
logged the expected zombie-to-missing identity transition after the manual TERM.
This was a manual deadline-gate stop, not a numerical refusal or deadline timeout.
The OS exit code is unrecorded because the manual group termination left the
`time.txt` file empty; no exit witness is synthesized.

`snapshot-inventory.tsv` is the result of a read-only pass over all 40 remote
snapshot bundles. The pass parsed every attempt and record, verified the exact
piecewise clock sequence, identity, committed outcome, 1,366,033,529-byte state
file, 12 RHS calls, cache `[7,5]`, zero steady allocations, finite observations,
and accepted error ratios. It also SHA-256 hashed every state, attempt, and record
file. `metadata/` preserves the lightweight records and raw logs; state payloads
remain on Sulaco under `/tmp/nsbu-p10-sulaco-m512-prefix-r5/run/output`.

This run is feasibility evidence only. It is incomplete at the original clock
4096 endpoint and makes no endpoint or PDE-window qualification claim.
