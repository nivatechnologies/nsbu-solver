# N512/M512 matched spatial endpoint preparation

This packet prepares a fresh-from-rest N512/M512 Cox--Matthews trajectory and
does not authorize or launch it. The source is the final tested FFT revision
`9eba11f196a25f0843f0cbd0f4ed08c9f7ae4645`, whose production implementation
is `0843b8b18e6a096a0208e3d896e391c7b1b2f5e0`. The harness is the existing
scheduled endpoint producer with the additive feature
`n512-m512-piecewise-cadv33`.

The profile starts from exact rest on the `2^-20` clock with target 8192. It
uses 32 steps of 64 ticks through clock 2048 and 16 steps of 128 ticks through
clock 4096, Cox--Matthews, absolute tolerances `[1e-5, 1e-4]`, relative
tolerances `[1e-5, 1e-5]`, and advective limit 3.3. The evolved state is never
reset. Each of the 48 accepted proposals is staged before commit and published
as a create-new, synced state bundle after commit. This preserves the actual
states required by later off-stage and reconstruction diagnostics. Clocks 512,
1024, 1536, 2048, 2560, 3072, 3584, and 4096 are marked as the offline observer
schedule.

The exact endpoint API reservation is 207,578,085,872 bytes. Its classes are
38,805,700,608; 32,614,907,904; 10,899,947,520; 3,233,808,384;
29,362,480; 91,810,835,512; 30,182,212,728; and 1,310,736 bytes. The
one-byte-under cap is refused before allocation. The harness fixes force W3 to
the M512 forward layout and RHS W3 to the padded M768 bidirectional layout.

The existing full physical observer needs 232,283,988,248 additional bytes at
the required 2N=1024 diagnostic geometry. Its combined exact API peak with the
trajectory owners is 439,862,074,120 bytes, so it cannot run inline under
Sulaco's 256 GiB address-space limit. The harness records no zero or substitute
observables: it marks observer work as pending and qualification false. The 48
actual states are to be archived and then observed on baccus under a separately
reviewed exact resource envelope.

The exact state-bundle disk bound is 155,226,537,984 bytes. The proposed source
and destination free-space floor is 189,586,276,352 bytes (the exact bound plus
32 GiB). Archival must stream into a create-new partial destination, compute
sorted SHA-256 inventories on both hosts, compare them byte-for-byte, sync the
destination, and rename it only after all 48 bundles and terminal metadata are
present. At an assumed 100 MiB/s lower transfer rate, the exact bound takes
about 1,481 seconds; this is an estimate, not observed N512 archive evidence.

Sulaco's completed 0-to-64 timing was 954.894656332 seconds of integration and
1051.20 seconds wall time, with 12 RHS calls, five misses, seven hits, and zero
steady allocations. The conservative run formula is
`ceil(1.25 * 48 * 1051.20 + 3600) = 66,672 seconds` (18:31:12), including an
hour for archive and guard work. Request a 20-hour block so a fixed absolute
deadline can be inserted into the reviewed v3 identity watchdog. The estimate
uses only the first step; later-step, state-write, archive-throughput, and
offline-observer timings are still missing.

`frozen-plan.json` is the review surface. A launch bundle is intentionally not
emitted because the current 2026-09-13T19:52:54Z deadline cannot contain the
run. After a new absolute deadline is authorized, freeze the binary, preflight,
plan, watchdog, and launcher hashes; retain the r6 startup identity capture,
process-group watchdog, signal cleanup, first-step checks, sole trajectory
attempt, fresh output path, host floor, disk floor, and durable archive guards.

Those watchdog, host, time, memory, disk, first-step continuation, and archive
checks are proposed wrapper requirements; no new v3 launcher exists or has been
executed. The checks executed during preparation are limited to the harness
tests, Clippy, build, exact API preflight, feature-exclusion compile refusal,
and opt-in run-gate refusal preserved under `raw/`.

No endpoint attempt, state allocation, state write, or observer run was made by
this preparation. Passing the preflight and focused harness tests does not
qualify a PDE window.

The reused harness `main.rs` and `config.rs` exceed 500 lines. This preparation
keeps the established transaction and scheduling structure intact rather than
splitting historical harness files during a run-critical adaptation. The N512
dead-code allowances are scoped to the individual balance, observer, owners,
records, and artifact modules that remain compiled to expose the offline
resource API; there is no crate-wide dead-code allowance.
