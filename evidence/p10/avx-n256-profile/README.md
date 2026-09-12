# N256/M384 accelerated profile admission and startup probe

This profile is distinct from the running N192 endpoint. Its endpoint admission retains the same
h32/max128 clock and observer nodes, with integration force M384, explicit observer force M768,
and conservative products on 2N=512. The one-step probe intentionally evaluates the current
observer at startup clock 32 to attribute integration, observer force, conservative, and transfer
cost. That extra startup observation is timing evidence only and is not a scheduled endpoint node.

The exact preflight must pass before the probe launches. The profile is experimental, nonresumable,
and provides no arithmetic, spatial, temporal, quadrature, or PDE-window qualification.

The frozen one-step source is the existing reviewed harness at
`evidence/p10/avx-parallel-reduced-composite-7467e26/harness/src/bin/cached_observed.rs`.
It runs with arguments `256 384 32 32 1 run96`; the literal mode selects the frozen
103,079,215,104-byte cap. The launch record binds the source commit,
binary hash, profile hash, host headroom, and exact output. The endpoint admission adds the fixed
1,572,864-byte snapshot/history allowance to that one-step owner's reservation.

The separately built full endpoint uses the cleared scheduled harness with the
`n256` Cargo feature. That feature fixes the retained layout and embeds
`profile=n256-m384` in preflight and artifact identity; the default feature set
continues to describe N192. Both profiles retain M384, the same schedule, and
the same transaction and refusal semantics. A full endpoint launch is allowed
only after this one-step timing supports completion before the experiment cutoff.

## Contended startup profile and endpoint launch

The exact `2593553ea6b616a0608694e67f0408702ae802b2` probe ran while the N192
endpoint was active. Its one accepted startup-ramp attempt took
184.520150132 seconds for integration and 158.601201704 seconds for the current
observer: 102.187050541 seconds force, 42.253849017 seconds conservative, and
14.160300026 seconds transfer/measurement. The total was 343.257658166 seconds,
with zero allocator calls in both instrumented regions and peak RSS
63,331,328 KiB. The startup state is a timing control and is not representative
of the concentrating endpoint.

At eight positive observer nodes in 128 attempts, this gives a conservative
measured-cost projection of 24,887.388831 seconds (6.913 hours), before the
small snapshot publication charge. The separately frozen endpoint source is
`81bd07ec0e52a16044e2814bf0dbf9711669830e`; its binary SHA-256 is
`6d939d4eef61d9b4cfd303319eafe689349eb82ec0801db11a053831427c6c27`.
Preflight admits 74,924,391,544 bytes under the 103,079,215,104-byte cap and
3,652,751,360 disk bytes under 4 GiB. The active `-b` launch started at
2026-09-12T16:39:43Z with timeout PID 430765 and solver PID 430766. Its exact
30,000-second timeout expires at 2026-09-13T00:59:43Z; the startup projection
ends near 23:34:30Z.

The original `81bd07e` launch refused before numerical work because its output
directory had been pre-created. The `-b` solver survived after its GNU-time
parent was lost during launcher cleanup. An accidental duplicate `-c` was then
terminated before node zero. Both records are retained. A fixed sidecar polls
the surviving solver's RSS, VmHWM, CPU ticks, and wall clock every 30 seconds.
Its maximum is an observed polled high-water mark, not a guaranteed final peak;
the missing GNU-time result remains disclosed and preflight is authoritative.

## Target-specific W3 projection

The independent W3 spike measured each three-transform layout-384 forward batch
at 2.149955551 seconds serial and 0.935004656 seconds parallel including caller
copy; inverse was 2.165047028 and 0.969849215 seconds. Applying only the 12
forward and 24 inverse rotational batches per attempt saves 43.264158252 seconds,
or 22.2515% of the measured scheduled cost of 194.4327252385 seconds. That misses
the 25% gate. Applying the same fixed owner to the five cached-force forward
triplets saves 49.338912727 seconds total, a projected 25.3758% reduction. This
is a component projection across separate measurements, not a measured composite
speedup, and clears the gate by only 0.3758 percentage points.

The reviewable candidate is one opt-in, length-384, width-three owner shared by
the accelerated RHS and reduced force provider. It owns three AVX plans and
workspaces, committed and staging buffers, and three persistent 2 MiB-stack
workers. Calls submit lanes 0/1/2, drain all completions including error or
caught panic, then publish in lane order. Failure poisons the owner; `Drop`
closes and joins every worker. Backend, supported length, width, worker count,
threshold, and cap enter immutable execution identity. The exact incremental
bidirectional reservation is 2,733,911,936 bytes; a less desirable pair of
separate rotational and force owners would require 4,561,854,080 bytes. The
measured loop had zero steady allocations and reproduced every serial result
word. No W3 production code is added until review because the projected margin
is narrow and must be confirmed by a composite N256 step.
