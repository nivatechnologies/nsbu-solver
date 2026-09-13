# Fresh M512 endpoint r6 evidence

The fresh-from-rest Sulaco trajectory completed all 48 scheduled integration
steps through clock 4096 and exited with status zero after 5:47:52. The final
coefficient hash is
`1d1500409962c4af37182f6733c2c88a086247c8e8728570e9c93238ed1c04fd`.
`final-run.json`, `clock-3584.json`, and `clock-4096.json` bind the terminal
status, timings, identities, and final hashes. The clock 512, 2048, and 3072
coefficient hashes exactly matched the prior r5 trajectory at those prefixes.

The actual independently evolved M384 trajectory now has M512 force comparisons
at every nonzero observer clock. Earlier r5 M512 results at clocks
[512](../snapshot-comparison-adapter/review-force-clock0512),
[1024](../snapshot-comparison-adapter/review-force-clock1024),
[1536](../snapshot-comparison-adapter/review-force-clock1536),
[2048](../snapshot-comparison-adapter/review-force-clock2048), and
[3072](../snapshot-comparison-adapter/review-force-clock3072) are preserved in
the adapter evidence. This directory adds r6 comparisons at clock 2560 in
`force-comparison-clock2560-run/` and clocks 3584 and 4096 in
`force-comparison-run/`. `force-comparison-results.json` records the combined
eight-clock inventory. All three new diagnostics exited zero under the reviewed
2,733,113,344-byte internal cap and 8 GiB external virtual-memory cap.

`durable-archive-receipt.json` binds the complete trajectory archive at
`/mnt/niva-array/nsbu-solver/work/p10-m512-endpoint-r6-artifacts-20260913`.
Its sorted local and remote SHA-256 inventories match for all 48 state files and
98 metadata files, totaling 65,569,755,172 bytes. The large state files remain
outside Git.

`ownership.json`, `first-step.json`, the checkpoint files, the NUMA samples,
and the clock-prefix files preserve observations made during the run. The
manifest files and launch scripts are the frozen inputs used for the post-exit
diagnostics. `adapter-build-binding.json` and the adjacent raw logs bind the
exact adapter source, binary, tests, Clippy check, and release build.

The endpoint and comparison outputs remain experimental. Every comparison
reports `accepted_windows: 0` and `acceptance.status: not_assessed`; they do not
establish PDE force-channel qualification or an accepted window. The analytical
projection was excluded as a trajectory counterpart. The second NUMA sample was
taken immediately after the clock-512 observer rather than during it.

The executed two-clock launcher checked for two stable exact-command samples,
but lacked a final counter assertion, leaving a theoretical single-sample edge
on its last polling iteration. Its observed launch receipts match the expected
command hashes. The later clock-2560 launcher added the explicit assertion and
completed normally.

`local-ratio-summary.json` aggregates all 48 archived attempt records. The
startup 64-tick segment reaches H1 ratio 0.963602 at clocks 192–256, close to
the local rejection threshold of 1. The later 128-tick segment reaches
0.007762 at clocks 3968–4096. These observed ratios do not establish temporal
convergence or justify enlarging the matched spatial comparison schedule.

The [timing distribution](timing-distribution.json) covers all 48 archived attempt
records with their hashes. Mean integration cost was 4.78% above the first step;
the slowest was 12.52% above it. Integration excludes observation and durable
publication. These older N384 measurements inform contingency planning, but
do not establish later-step N512 runtime or acceptance.
