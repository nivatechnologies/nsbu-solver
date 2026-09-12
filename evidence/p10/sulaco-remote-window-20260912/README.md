# Sulaco extended-window numerical evidence

This directory retains the small records from the bounded Sulaco experiments on
2026-09-12. Large coefficient and state files remain in the ignored local archive at
`work/p10-sulaco-remote-20260912`; their hash inventories are committed here. None of the
accelerated results qualifies the arithmetic or supports a cross-host bit-equivalence claim.

## Exact-source guard trajectories

The exact-source N96/M192 CM h32/Cadv0.3 endpoint completed all 128 attempts at clock
4096 in 17,073.967 s with maximum RSS 5,683,200 KiB and zero swap. It reported no
rejection or refusal. The 21,676,032-byte terminal coefficient file hashes to
`4d250ccd8e9d386c99d0402b8c1ca2ffc75e54a265cf0924aa3b98087ea6d1a8`.
Its same-profile spatial comparison with N128 remains deferred until the independent N128
endpoint completes.

The completed direct-from-rest N96/M192 CM h64 and h32 states differ by
`9.018521337154418e-10` L2, `1.2125912739538688e-7` H1,
`1.2125577363343584e-7` vorticity L2, and `9.840765199076487e-15`
unprojected-divergence L2. Relative to the h32 state, the first three ratios are
`4.904300604634981e-10`, `2.384208344535477e-9`, and
`2.385702329661314e-9`. These use the same source, case, grid, force sampling,
method, tolerances, advective limit, workers, clock, and from-rest initial condition. This is
still a two-setting time comparison and does not fulfill the required three-setting study.

The exact `1b89176f63737bbeb3ed65c7e3c7fe6dbeef8f02` N128/M192 trajectories both
completed 64 h64 steps at active advective limit 0.45. CM completed in 11,291.427 s with
maximum RSS 7,989,248 KiB; HO completed in 13,364.992 s with maximum RSS 8,470,528 KiB.
Both committed all attempts without a rejection or numerical/resource refusal. The terminal
coefficient hashes are `d2d2289956c398df328b599f8ed0e2e6f8ab38bbb3086bbd8fb7afc5e8df87fd`
for CM and `93b3a483a7c98f1ca05a77c742d67388dbe9ad1cbeef94aa22595dd163ce7cdf`
for HO.

The hash-bound same-profile comparison reports L2 difference
`9.486263244459316e-10`, H1 difference `1.4285421756356483e-7`, vorticity-L2
difference `1.4285106784321692e-7`, and divergence-L2 difference
`8.201781182123754e-15`. Comparison with the N128/M192 CM h32/Cadv0.3 endpoint is
deferred until that independent endpoint completes.

## Accelerated N192 endpoint

The separately identified `n192-m384-h64-cadv08` profile completed 64 attempts and eight
positive-node M768 observations at clock 4096. Its source identity is base commit
`81bd07ec0e52a16044e2814bf0dbf9711669830e` plus source-manifest SHA-256
`66a99d6d22c56202ccbd9d4a2d68c184a465157c8ac7cfa44661e225d77737ee`.
The run took 2:00:39, used at most 45,166,592 KiB according to GNU time, and reported zero
steady allocations. The terminal logical state hash is
`a3e93565644a4d3e4f1131d17994e6c80ef5e18ccd83c42809466157ddb66aec`.
Its later comparison with the local exact N192 h32 trajectory must disclose different hardware
and arithmetic; the pair cannot isolate a time-step effect.

## N384 h32 pilot

The source `68755e0797b607c7e79e2ad3700cfc33e6b66fd4` h32 pilot passed frozen memory and
disk admission, then committed one from-rest step at clock 32. Integration took
352.878228511 s: 314.081122781 s in 12 timed RHS calls and 38.797105730 s outside RHS.
The 1,366,033,465-byte state file hashes to
`d993e3618627bf7e804e68a47912096dfbb64e4601fd82177a2c5a7e24a8ad3a`.
Clock 32 has no scheduled observer. The complete bundle was hash-bound before an externally
requested whole-process-group TERM, so it is retained as provisional.

The binary's immutable identity text names
`external_stop=pgid-watchdog-v1-starttime-cmdline-deadline`. That text accurately identifies
the binary that ran and was not rewritten. The external protection changed during execution:
the frozen v1 script used shell `$20`, which expanded as positional parameter 2 plus a literal
zero instead of parsing `/proc/PID/stat` field 22. It exited at 18:59:17Z after the GNU-time
leader was reparented, leaving a disclosed protection gap. Corrected watchdog v2, SHA-256
`4b65e13b74bd7a32237044b5467d4f223c8dc3e6e35d6e8d3498e1938fa53e4b`, was
attached first to the wrapper and then directly to solver PID 175223 at 19:05:26Z. The direct
attachment bound PGID 175221, true starttime 31557951, command-line SHA-256
`e78e13ed4af29892749369b7114badd02f6cbfc93292add78f2aff3ca2575d83`, and the
original deadline. The full incident and v2 validation are recorded in
`../avx-n384-endpoint-prep/watchdog-v2-amendment.md`.

The planned process-group stop also signaled the GNU-time leader, leaving `time.txt` empty.
Formal VmHWM and process-specific NUMA placement were therefore unavailable after exit. Repeated
live measurements reached 148,651,168 KiB RSS with zero swap; this is an observed value rather
than a formal high-water mark. No per-thread CPU distribution was captured. Those missing data
provide no concrete basis for a placement-only claim of at least 20% RHS improvement.

Even before eight unknown later observer costs, 128 measured h32 attempts plus measured snapshot
publication and one-time construction project to at least 46,092 s. A stated 25% growth and
contention margin raises that to about 57,596 s plus the eight observers. The analogous raw h64
floor was already about 23,085 s plus eight observers, beyond the remaining window. The h64 pilot
was therefore left unlaunched at that decision point.

## N384 piecewise Cadv1.6 superseded run

A later, separately frozen N384/M384 piecewise profile scheduled h64 through clock 2048 and
h128 through clock 4096 under active Cadv1.6. It committed three full state bundles through
clock 192 without a numerical or resource refusal. The integrations took 317.463, 378.405,
and 370.278 seconds. Live resource records reached 148,652,256 KiB RSS/HWM with zero swap.

After the three-step mean placed a conservative endpoint projection at essentially the 01:25
watchdog, root authorized a corrected higher-guard supersession. The solver PID, process group,
true starttime, and command-line hash were verified before TERM at 19:39:24Z; it exited without
KILL at 19:39:37Z. This is a manual guard-based supersession, not an actual advective or resource
refusal. The three 1,366,033,529-byte encoded states and every small record were transferred to
the ignored local archive and verified against the committed hash inventory.
