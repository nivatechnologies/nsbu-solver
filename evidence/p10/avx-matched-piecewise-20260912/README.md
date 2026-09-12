# Frozen matched N192/N256 piecewise profiles

These two experimental profiles are frozen from source
`569fd0ced7a755b1f6c066a7c00016f537fbc4bf`; neither profile was launched by this
preparation task. Both use Cox--Matthews with 32 steps of 64 ticks on `[0,2048)`
and 16 steps of 128 ticks on `[2048,4096)`, maximum 48 attempts, advective guard
3.3, unchanged absolute tolerances `[1e-5,1e-4]`, and unchanged relative
tolerances `[1e-5,1e-5]`.

Both profiles use integration force grid M384 and the finite RustFFT 6.4.1
AVX/AVX2/FMA backend. N192 uses serial component transforms for padded RHS layout
288, which is outside the validated W3 layout set, plus the validated three-lane
W3 pool for M384 force transforms. N256 uses separate validated three-lane W3
pools for padded RHS layout 384 and M384 force transforms. The force sampler has
32 persistent sampling workers in both profiles.

| Field | N192 | N256 |
|---|---:|---:|
| Exact execution reservation | 52,428,314,504 B | 79,485,917,960 B |
| Execution cap | 206,158,430,208 B | 206,158,430,208 B |
| Cap margin | 153,730,115,704 B | 126,672,512,248 B |
| Integration work bound | 4,213,502,118,720 | 4,221,931,883,328 |
| Observer work bound | 467,480,346,624 | 467,480,346,624 |
| Artifact bound | 8,242,397,184 B | 19,482,083,328 B |
| Artifact cap | 137,438,953,472 B | 137,438,953,472 B |
| Snapshot bundle bound per accepted step | 171,716,608 B | 405,876,736 B |

The observer force grid is fixed at 768 at clocks 0, 512, ..., 4096. The
conservative diagnostic retains its existing `2N` policy: grid 384 for N192 and
grid 512 for N256. The resulting diagnostics are grid dependent. They do not
support a complete common-grid pressure or regional spatial comparison. A fixed
physical grid 768 conservative diagnostic can be computed offline from the
captured states under a separately identified analysis profile.

REST is metadata only. Every accepted positive proposal stages a full
nonresumable state snapshot and attempt record before the in-memory commit.
Unscheduled bundles explicitly carry `observation_status=NotScheduled` and no
balance. Scheduled bundles add the actual balance. Rename plus parent-directory
sync confirms the durable frontier. Rename failure may retain `.partial`; a
parent sync failure after rename may leave the final path present but unconfirmed.
Failure output distinguishes the attempted, in-memory committed, confirmed
durable, and provisional frontiers and reports either possible bundle location.

## Local placement

At `2026-09-12T20:27:55Z`, the local host reported 527,922,496 kB total and
440,720,124 kB available memory, with all 8,388,604 kB swap free. The former N192
job had exited. The protected older N256 job remained as PID 430766 in PGID
430765 with 63,335,844 kB resident/high-water memory and its existing
`2026-09-13T00:59:43Z` timeout. The output filesystem had
7,268,798,742,528 bytes available.

N192 may replace the completed N192 slot while the protected N256 continues. A
fresh launch-time check must require at least its exact 52,428,314,504-byte
reservation plus a 32 GiB OS cushion in `MemAvailable`; the running N256 is
already reflected in that kernel value. Refuse below 86,788,052,872 bytes or if
filesystem availability is below the 128 GiB artifact cap. Use the frozen
PGID/starttime/cmdline-validating watchdog and a deadline no later than
`2026-09-13T01:20:00Z`, followed by KILL after 60 seconds if the exact identity
survives.

The matched N256 profile should wait for the protected old N256 and matched N192
to finish. Do not overlap both new profiles: each has an immutable 192 GiB
execution cap even though its exact reservation is lower. Its fresh admission
floor is 79,485,917,960 bytes plus the same 32 GiB OS cushion, or
113,845,656,328 bytes available, and 128 GiB of free artifact space. Within the
current experiment window, refuse a launch whose measured projection cannot
finish before the hard stop with margin.

## Verification and identity

Each exact feature passed 19 focused tests, `cargo fmt --check`, and strict
all-target Clippy. Tests cover all 48 schedule intervals and the 2048 transition,
all fine observer nodes, invalid/end refusal, exact reservations, and one-byte-
below-cap refusal. Static analysis reports maximum function cyclomatic complexity
16, cognitive complexity 7, Halstead difficulty 26.833, and maximum source file
length 499 lines. No new endpoint timing or trajectory claim is made here.

The raw preflight records and frozen plans are in this directory. Release
binaries remain outside Git under
`deployment/569fd0ced7a755b1f6c066a7c00016f537fbc4bf/`; the deployment manifest
fixes their exact names and SHA-256 digests. The required watchdog is
`evidence/p10/avx-n384-endpoint-prep/pgid-watchdog-v2.sh`, SHA-256
`4b65e13b74bd7a32237044b5467d4f223c8dc3e6e35d6e8d3498e1938fa53e4b`.
