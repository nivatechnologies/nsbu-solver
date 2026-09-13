# Reviewed N384/M512 matched force-refinement deployment

This bundle freezes a from-rest N384 trajectory with an M512 integration-force
grid. It is prepared for root review and has not been launched. The current
Sulaco N384/M384 trajectory must finish and release the whole host before this
candidate can be admitted.

The source is commit `326eeb5cbd5ebe39a7d5f7be77f9acfab8d0db72` on
`codex/p10-m512-force-profile-20260912`. The source-bound binary uses feature
`n384-m512-piecewise-cadv33`, RustFFT 6.4.1 AVX/AVX2/FMA, the
parallel-reduced-v2 W3 attempt-cache provider, bidirectional layout-576 RHS W3,
and forward layout-512 force W3. Retained state is N384. Observer force and
conservative layouts remain 768.

The exact schedule is 32 steps of 64 ticks through clock 2048 and 16 steps of
128 ticks through clock 4096. Cox--Matthews, Cadv 3.3, absolute tolerances
`[1e-5,1e-4]`, relative tolerances `[1e-5,1e-5]`, the nine existing observer
nodes, and a full nonresumable snapshot after every accepted step are retained.

Preflight reserves 193,243,071,240 bytes under the 206,158,430,208-byte cap,
leaving 12,915,358,968 bytes. Artifacts are bounded at 65,573,289,984 bytes
under the 137,438,953,472-byte cap. Admission requires the prior N384 process
to exit successfully, no other solver on Sulaco, `MemAvailable` at least the
full execution cap, and output-filesystem availability at least the full
artifact cap immediately before construction.

The measured central forecast is 18,919.87 seconds. It combines the current
N384/M384 integration and observer means through attempt 38 with five measured
force misses per step and the independent M512-minus-M384 force increment.
After a 600-second startup allowance and 15% margin, the frozen budget is
22,448 seconds (06:14:08). Launch must begin by 2026-09-13T00:45:52Z. The
first accepted step must take at most 399 integration seconds and leave at
least 21,359 seconds before the numerical deadline, along with exact identity,
12 RHS calls, cache `[7,5]`, and zero steady allocations.

The original full-run plan used true-v2 and remains immutable historical
provenance. The current operational launcher uses reviewed v3, bound to the
solver PID, process group, starttime, and full command-line SHA-256. Its numerical deadline is
2026-09-13T07:00:00Z; it sends TERM to the solver process group and KILL after
60 seconds if the same identity remains. The hard block is
2026-09-13T07:16:47Z. Root authorization is still required. Running preflight,
building this bundle, or reviewing its hashes does not authorize launch.

The final launcher amendment arms failure traps before spawn. During the
`setsid` transition, cleanup is restricted to the direct child or a process
group whose ID is that exact child PID. Handoff from this transitional state
requires a stable starttime, the expected new process group, and the exact GNU
time/timeout wrapper command. GNU timeout independently applies TERM and KILL
from the remaining absolute-deadline budget. The launcher then binds the actual
solver grandchild and requires the identity-bound v3 watchdog's exact `started`
record and live non-zombie state through the first-step gate. Dummy controls
cover the pre-setsid transition, immediate post-spawn race, capture exit 80,
child-group validation exit 81, and watchdog attachment exit 96. The numerical
binary and frozen plan were not rebuilt or changed.

Launch admission also performs complete observer integrity checks at the eight
non-rest predecessor nodes from clocks 512 through 4096. It requires exact
N384/M384 source and profile identity, committed attempt and observation
records, the expected snapshot size, and exact RHS/cache/allocation counters.
Norms, energies, dissipations, timings, and error ratios must be finite and
nonnegative. The signed forcing-work, stretching, and vorticity-forcing
channels must be finite; their sign is preserved.

The final inert archive was staged at
`/tmp/nsbu-p10-sulaco-m512-20260913-r2` on Sulaco and all seven runtime file
hashes passed. At staging time the predecessor was still the sole solver,
`MemAvailable` was 97,695,559,680 bytes (108,462,870,528 below the future
admission floor), filesystem availability was 1,708,243,427,328 bytes, and the
new bundle had no `run` directory. Same-host preflight and launch were not
executed.

The active v3 watchdog corrects the reviewed transient-sampling control-flow
defect while retaining v2 unchanged. Initial attachment mismatch diagnostics and all later component/observed
identity diagnostics go to the watchdog audit log. Its dummy control logged an intentional
cmdline-hash mismatch with exact expected and observed identity, recovered on
the confirmation sample, and sent no TERM. A separate control sent TERM only
after the explicit deadline. V3 became ready at 00:45:08Z; safe review and
staging could not complete before the frozen 00:45:52Z latest start, so no v3
trajectory was launched.
