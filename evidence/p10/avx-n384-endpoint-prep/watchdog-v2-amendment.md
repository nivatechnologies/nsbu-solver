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

At `2026-09-12T19:03:59Z`, v2 was attached without restarting the live h32
pilot. It matched leader PID/PGID `175221`, true starttime `31557951`, and
command-line SHA-256
`2465ed18b23a638842f4420873fa0d5fc45edb1d9c869585d08a95fad9582901`.
The original deadline epoch `1789241327` remained unchanged. This attachment
restored identity-bound deadline protection for the pilot. New pilot or
endpoint launches must use v2; the h32 numerical profile itself is unchanged.
