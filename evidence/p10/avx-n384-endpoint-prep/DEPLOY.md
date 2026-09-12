# Deployment procedure

This bundle is eligible only for an explicitly authorized Sulaco pilot. Verify every file against
`deployment-manifest.json` and select h32 first. Preserve any hash, CPU admission, memory, disk,
artifact-cap, or time-fit refusal. H64 is a distinct fallback; do not alter h32 files or outputs.

Immediately before construction, capture `date -u`, `uname -a`, `lscpu`, `numactl --show`,
`/proc/meminfo`, `df -B1` for the output filesystem, active numerical processes, and their frozen
reservations. Require available memory after other reservations to meet 206,158,430,208 bytes.
Require available disk to meet the selected plan's full artifact cap. Run the binary's `preflight`
mode and compare its full output and SHA-256 with the frozen copy before creating the output path.

Launch the selected binary without CPU or memory binding so all Sulaco CPUs and memory nodes remain
visible. Make GNU time the process-group leader:

```sh
setsid /usr/bin/time -v -o "$RUN/time.txt" "$BINARY" run "$RUN/output" \
  >"$RUN/stdout" 2>"$RUN/stderr" &
leader_pid=$!
```

Before attaching the watchdog, record the leader process group, `/proc/$leader_pid/stat` starttime,
and SHA-256 of `/proc/$leader_pid/cmdline`. Freeze an absolute UTC deadline no more than 1,800
seconds after launch. Start the watchdog in its own session and record its PID, start time, script
hash, exact arguments, and log path:

```sh
setsid ./pgid-watchdog-v2.sh "$leader_pid" "$process_group" "$starttime" \
  "$cmdline_sha256" "$deadline_epoch" "$RUN/watchdog.log" &
watchdog_pid=$!
```

Use `pgid-watchdog-v2.sh` for every launch. The frozen v1 script is retained
only to reproduce the first h32 pilot's operational failure: its unbraced
`$20` expanded as `${2}0` in POSIX shell and did not compare `/proc` field 22.
The [watchdog amendment](watchdog-v2-amendment.md) records the repair and its
focused controls.

For h32, observe `step-001-clock-0032`; for h64, observe `step-001-clock-0064`. After retaining the
complete final bundle, stop the whole process group and record the operator stop time. Classify the
bundle conservatively as provisional because external directory observation alone cannot prove that
the harness's parent-directory sync returned before the stop. Preserve the GNU-time result, stdout,
stderr, watchdog log, host admission, `/proc` identities, exact command, process IDs, all hashes,
and the attempt bundle. Do not launch a full endpoint until the pilot timing and remaining-time gate
are reviewed.
