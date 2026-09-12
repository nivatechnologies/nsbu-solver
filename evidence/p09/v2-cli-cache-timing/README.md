# CLI attempt-force-cache timing evidence

This source-bound archive preserves eight sequential Linux release-binary runs
from source `8bec567f9befffd085343850ddc53140251d9eb6`. The fixed profile was
N12, force grid M48, 32 workers, step ticks 16, endpoint ticks 128, maximum
attempts 8, and declared memory cap 256 MiB. Each method was run twice with
the direct force path and twice with `--cache-force`; every run exited 0,
reached tick 128, and committed 8/8 attempts with no refusals.

Measured wall times were:

| Method | Direct runs | Cached runs | Median direct/cached ratio |
|---|---:|---:|---:|
| CM | 39.57 s, 41.06 s | 24.42 s, 25.42 s | 1.619 |
| HO | 46.61 s, 48.04 s | 25.05 s, 24.96 s | 1.893 |

These ratios are measurements for this profile under shared host load, not
universal speed claims. Maximum resident set size was 68,608–70,656 KiB.
The paired core diagnostic JSON matched after excluding only the enumerated
cache/ledger fields in `summary.json`; terminal clocks, attempts and observation
ledgers matched, while integration ledger differences were expected.

This archive makes no state-bit claim and does not measure convergence or PDE
qualification. `commands.txt`, raw JSON, stderr timing, timestamps, exit codes,
source/binary/tool hashes and `SHA256SUMS` preserve the reproduction inputs.
