# Corrected first-endpoint resolution pilot

This archive preserves the bounded diagnostic run from exact-v2 rest at source
`487c3307037be09e19d151324a2bf96dbc92e83b`. It follows the earlier failed
pilot after the derivative staging repair. It is diagnostic evidence only: it
does not qualify a PDE window or establish convergence.

The fixed family used CM spatial grids 12, 16 and 24, time steps 64, 32 and 16,
and an HO grid-24/step-16 branch. Every ordinary branch used the original exact-v2
M24 force with 12 workers. Accepted clocks were 0, 2048 and 4096; offstage probes
were 2047 and 4095. Physical/reference sampling used 24 cubed points, while
pressure and residual diagnostics used the original M48 force and 48 cubed
points. The extra force diagnostic was one N12/CM/step-16 M24-to-M48 comparison;
it is not a three-level force refinement or evidence that M48 is sufficient.

The allocation-only `--dry-run` admitted 1,514,546,011 bytes under the 4 GiB
cap. This includes the 1,393,212,608-byte coordinator joint reservation, the
120,898,776-byte independent M48 run, a 429,979-byte reusable archive buffer,
552 bytes of comparison/hash scratch and a 4,096-byte I/O allowance. The earlier
`--preflight` name denotes a real short numerical walkthrough to endpoint 128,
with accepted clocks 0/64/128, probes 0/63/64/127/128 and the same step sizes. It
completed in 214.28 seconds with 581,632 KiB maximum RSS. It ran at source
`489896369679f35a545df26e5ed9c534d27ed24b`; the archived check shows no production
crate diff between that source and 487c330.

The full command started at 02:07:22 UTC and hit its unchanged 6,000-second
timeout at 03:47:22 UTC, status 124. It published complete events through probe
4095. The accepted-4096 coordinator call and its consumers returned, as shown by
the harness control flow and all six subsequent branch archives, but timeout
occurred inside the separate M48 control before event 5 was printed. Therefore
the run is partial: it does not contain the accepted-4096 physical, pressure or
reference report, the endpoint force comparison, or a complete terminal event.
The `/usr/bin/time` process was terminated by the outer timeout, so no full-run
maximum-RSS record is available.

An archive-only reader rebuilt every expected branch plan, checked archive hashes,
identities, clocks and byte bounds, and compared the retained endpoint states. It
completed in 0.03 seconds at 3,076 KiB maximum RSS with a conservative
194,348,014-byte maximum pair reservation. Endpoint full-band norms were:

| comparison | L2 | H1 | vorticity L2 |
|---|---:|---:|---:|
| CM N12 to N16, step 16 | 0.46210490660004033 | 22.213794282100803 | 22.208987267833496 |
| CM N16 to N24, step 16 | 0.39051181863199025 | 26.79566622262152 | 26.79282047179189 |
| CM N24, step 64 to 32 | 6.561621651540645e-10 | 1.517996590705549e-8 | 1.5165777795823317e-8 |
| CM N24, step 32 to 16 | 4.1177065185446655e-11 | 9.603999231301006e-10 | 9.595167853441537e-10 |
| CM to HO, N24 step 16 | 2.1407145059886384e-12 | 5.936119038255797e-11 | 5.932257458005536e-11 |

L2 decreases from the first to the second spatial pair, while H1 increases. This
repeats the qualitative nonmonotone H1 behavior in the prior N8/12/16 pilot, at
smaller absolute differences. The cross-pilot absolute change is confounded by
the prior M16/eight-worker and current M24/12-worker force settings, so it is not
an isolated spatial improvement. It does not demonstrate convergence. At clock 2048,
the one M24-to-M48 force comparison had L2 0.3332248325130536 and H1
9.868231645532276. An independent cached M48 run with 32 workers matched this
run's 12-worker M48 coefficient-only hash exactly at 2048; that verifies state
word equality for that branch and clock, separate from record equality.

The complete clock-2047 probe/residual record equals the failed baseline record
byte for byte after removing only wall time. The normalized SHA-256 is
`a035e93b4b6b69d7bde45b2ad4edd055b010d6f95c89396e0ca5b6406877362f`.
The old run retained no matching state archive, so this does not establish state
bit equality.

`raw/` contains deterministic gzip copies of all logs, commands, source markers
and checks. `checkpoints/` contains all 18 raw branch archives as deterministic
gzip streams. `artifact-sha256.json` binds every retained artifact.
