# Frozen N384 accelerated pilot profiles

This directory freezes two separately identified, experimental N384 profiles without launching
either one. Both use source `68755e0797b607c7e79e2ad3700cfc33e6b66fd4`, integrated base
`29d9f1c`, and reviewed W3 source `f13c29c9ae91d0b8cf7a790132deb9bd076911c0`.
`frozen-plan-h32.json` is the first-choice pilot. `frozen-plan-h64.json` is the approved fallback
if the h32 pilot does not fit the remaining window or is refused. A full endpoint is not authorized
until a same-host pilot supplies an actual N384 step cost and the remaining-time gate passes.

Both profiles retain N384, sample the cached reduced integration force at M384 with 32 sampling
workers, and keep the existing tolerances and actual-stage advective limit 0.8. They use the finite
RustFFT 6.4.1 AVX/AVX2/FMA catalog, a distinct three-worker W3 pool for padded RHS layout 576, and a
distinct three-worker W3 pool for M384 force transforms. The exact persistent W3 additions are
9,200,779,136 and 1,827,942,144 bytes. Construction checks backend, layout, lane count, direction,
and storage identity before execution.

The only profile difference is the integration step and resulting attempt/artifact bound:

| Field | h32 | h64 fallback |
|---|---:|---:|
| Step ticks | 32 | 64 |
| Maximum attempts | 128 | 64 |
| Exact execution reservation | 185,783,161,608 B | 185,782,899,464 B |
| Execution cap | 206,158,430,208 B | 206,158,430,208 B |
| Execution-cap margin | 20,375,268,600 B | 20,375,530,744 B |
| Full artifact bound | 174,862,106,624 B | 87,431,053,312 B |
| Artifact cap | 274,877,906,944 B | 137,438,953,472 B |
| Frozen binary SHA-256 | `c58c04dfa9aef7adffb1234c314d9cbe2625f5abd7152b4f758fdb0415ec4882` | `5e60947ae82c2cf09bf4960fec869440343eb47d37324b5ac406c81b9878e5df` |

The h32 reservation classes are 16,392,388,608 B retained state/history, 13,759,414,272 B padded
real arrays, 4,602,396,672 B padded complex arrays, 1,366,032,384 B tables, 29,362,480 B shared FFT
catalog, 38,807,118,904 B RHS/cached force/sampling/W3 owners, 110,824,809,872 B attempt/observer
owners, and 1,638,416 B harness overhead. H64 changes only harness overhead to 1,376,272 B. One byte
below each exact total refuses.

The observer schedule is fixed at clocks 0, 512, 1024, 1536, 2048, 2560, 3072, 3584, and 4096.
Rest uses the exact analytic balance without a state payload. Each positive node evaluates observer
force M768 and conservative grid 768; no unscheduled balance is synthesized. The nested Simpson
levels remain a refinement test and make no spatial or quadrature sufficiency claim.

Every accepted positive proposal stages one full nonresumable N384 snapshot, its attempt record,
and its observation record before the infallible in-memory commit. Unscheduled records carry
`observation_status=NotScheduled` and no balance. Scheduled records carry the actual balance in the
same directory bundle. A successful rename plus parent-directory sync advances the confirmed
durable attempt and clock together. Rename failure can retain `.partial`; parent-sync failure after
rename can leave the final path unconfirmed. Failure status reports attempted, in-memory,
confirmed-durable, and provisional frontiers plus both possible paths. The protocol does not claim
crash atomicity across memory and filesystem.

The harness-local `TimedRhs` delegates all 12 RHS evaluations without changing arithmetic. It
records the inclusive RHS wall sum, call count, whole `try_advance` wall, and
`outside_rhs_evaluate_seconds`. The remainder includes `begin_attempt`, force-cache setup,
coefficient tables, validation, indicators, wrapper accounting, and other attempt work; it is not
an isolated coefficient-table measurement. The authoritative whole-attempt allocator region must
remain zero-allocation.

## Pilot admission and stopping

Execution is restricted to Sulaco with `whole-host-unbound-all-visible-cpus-memory`. Immediately
before construction, refuse unless `MemAvailable` minus other frozen reservations is at least the
192 GiB execution cap. Also require available bytes on the chosen output filesystem to meet the
selected artifact cap. Capture the host snapshot, exact command, source/binary/plan hashes, process
group, `/proc` starttime, command-line SHA-256, watchdog PID/start/deadline, and NUMA status.

The frozen one-step pilot outer timeout is 1,800 seconds for either profile. This is only a bounded
pilot allowance: N256 final-source timing cannot establish N384 endpoint cost. Stop the whole
process group after the first final step bundle is observed and retained. Because the source has no
external durable-frontier marker, an operator stop after observing the final directory must label
that bundle provisional unless normal process output independently establishes completion; it must
not silently claim a confirmed durable frontier. The deadline watchdog validates group-leader
starttime, process group, and command-line hash on every poll, sends TERM to the group at the fixed
deadline, and sends KILL after 60 seconds if the same identity survives. Its frozen SHA-256 is
`2e5afc6cadad3549bb947aface8e2a2ae2aa6594ebe1e1f289441834e0f113ab`.

No endpoint timeout is frozen. After the pilot, use the measured whole attempt, RHS split, snapshot
write, peak RSS, steady allocations, outcome, and any observer component measurement to decide
whether the endpoint can finish before the experimental cutoff with margin. Do not infer N384
cost by multiplying isolated AVX, reduced-force, or W3 speedups.

## Verification

Each exact feature passes 18 focused tests, formatting, and strict all-target Clippy. The W3 controls
also pass nine solver tests and four provider tests for finite admission, cap boundaries, serial bit
equality, worker drainage after failure/panic, permanent termination, and publication preservation.
Maximum function cyclomatic/cognitive complexity is 16/7, maximum function Halstead difficulty is
26.833, and the largest source file is 499 lines.

Branch-enabled LLVM coverage combined with the default-profile transactional tests is 995/2,157
lines and 37/64 branches for h32, and 994/2,156 lines and 37/64 branches for h64. Both exact reports
have maximum per-function CRAP 22.5 and zero violations. This focused coverage is reported honestly;
it is not the maintained whole-scope 80% coverage gate.

The frozen raw preflight lines are `h32-preflight.stdout` and `h64-preflight.stdout`. Release
binaries are intentionally outside Git; their names and hashes are fixed in the plans and deployment
manifest. The local handoff archive is
`deployment/p10-n384-68755e0-deployment.tar.gz`; its external archive digest and internal checksum
file are preserved as `deployment-archive.sha256` and `deployment-SHA256SUMS`.
