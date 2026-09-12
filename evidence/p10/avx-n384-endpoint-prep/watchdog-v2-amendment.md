# N384 watchdog v2 amendment

The first N384 h32 pilot exposed an operational defect in the frozen v1
watchdog. After stripping the PID and parenthesized `comm` field from
`/proc/PID/stat`, v1 used `$20`. POSIX shell parses that as `${2}0`, so it
compared the expected starttime with the PPID followed by `0`, rather than the
twentieth word of the remainder. That word is Linux `/proc/PID/stat` field 22,
the process starttime. The original v1 script and its failed remote log remain
part of the pilot evidence.

V2 changes only that expansion to `${20}` and documents the field mapping. It
retains the process-group, command-line hash, deadline, TERM, and KILL behavior.

- V1 SHA-256: `2e5afc6cadad3549bb947aface8e2a2ae2aa6594ebe1e1f289441834e0f113ab`
- V2 SHA-256: `4b65e13b74bd7a32237044b5467d4f223c8dc3e6e35d6e8d3498e1938fa53e4b`
- V2 syntax: `dash -n`, status 0
- V2 deadline control: a separate `setsid sleep 300` process group was
  identity-matched by true starttime and command-line hash, received TERM at a
  two-second deadline, and exited during grace with watchdog status 0.
- V2 mismatch control: an incorrect starttime refused before monitoring with
  status 65 and left the target process alive.

The v1 watchdog exited at `2026-09-12T18:59:17Z` while the pilot remained live.
There was no independent GNU `timeout` process, so the pilot had a disclosed
deadline-protection gap until v2 was attached at `2026-09-12T19:03:59Z`.
That first v2 attachment matched the `/usr/bin/time` wrapper. It was replaced
at `2026-09-12T19:05:26Z` by a direct solver attachment so protection does not
depend on wrapper lifetime. The final attachment matched solver PID `175223`,
process group `175221`, true starttime `31557951`, and command-line SHA-256
`e78e13ed4af29892749369b7114badd02f6cbfc93292add78f2aff3ca2575d83`.
The original deadline epoch `1789241327` remained unchanged. New pilot or
endpoint launches must use v2 bound directly to the solver PID; the h32
numerical profile itself is unchanged.
