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

The true v2 watchdog is bound to the solver PID, process group, starttime and
full command-line SHA-256. Its numerical deadline is
2026-09-13T07:00:00Z; it sends TERM to the solver process group and KILL after
60 seconds if the same identity remains. The hard block is
2026-09-13T07:16:47Z. Root authorization is still required. Running preflight,
building this bundle, or reviewing its hashes does not authorize launch.

The final launcher amendment installs identity-checked cleanup immediately
after the `setsid` leader is captured, so child-capture, child-group validation, or watchdog-attachment
failure cannot leave the owned solver group running. Handoff requires both the
watchdog's exact `started` record and a live watchdog PID. Dummy process-group
tests cover capture exit 80, child-group validation exit 81, and attachment exit 96. `launcher-amendment.json`
binds the preserved binary and plan hashes, the amended recipe, support source,
and test source. The numerical binary and frozen plan were not rebuilt or
changed for this amendment.

Launch admission also performs the complete eight-node predecessor screen at
clocks 512 through 4096. It requires exact N384/M384 source and profile
identity, committed attempt and observation records, the expected snapshot
size, exact RHS/cache/allocation counters, finite nonnegative timings and
balances, and accepted error ratios at every non-rest observer node.

The final inert archive was staged at
`/tmp/nsbu-p10-sulaco-m512-20260913` on Sulaco and all seven runtime file
hashes passed. At staging time the predecessor was still the sole solver,
`MemAvailable` was 97,720,217,600 bytes (108,438,212,608 below the future
admission floor), filesystem availability was 1,710,977,822,720 bytes, and the
new bundle had no `run` directory. Same-host preflight and launch were not
executed.
