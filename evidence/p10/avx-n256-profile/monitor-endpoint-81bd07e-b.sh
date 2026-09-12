#!/bin/sh
set -eu

solver_pid=430766
deadline_epoch=1789261183
output=evidence/p10/avx-n256-profile/endpoint-run-81bd07e-b.resource.csv

printf 'utc,elapsed_seconds,vmrss_kib,vmhwm_kib,user_ticks,system_ticks\n' > "$output"
started=$(date -u +%s)
while kill -0 "$solver_pid" 2>/dev/null && [ "$(date -u +%s)" -lt "$deadline_epoch" ]; do
    now=$(date -u +%s)
    utc=$(date -u +'%Y-%m-%dT%H:%M:%SZ')
    vmrss=$(awk '$1 == "VmRSS:" { print $2 }' "/proc/$solver_pid/status")
    vmhwm=$(awk '$1 == "VmHWM:" { print $2 }' "/proc/$solver_pid/status")
    set -- $(awk '{ print $14, $15 }' "/proc/$solver_pid/stat")
    printf '%s,%s,%s,%s,%s,%s\n' "$utc" "$((now - started))" "$vmrss" "$vmhwm" "$1" "$2" >> "$output"
    sleep 30
done
status=solver_exited
if kill -0 "$solver_pid" 2>/dev/null; then
    status=monitor_deadline_reached
fi
printf '%s,%s\n' "$(date -u +'%Y-%m-%dT%H:%M:%SZ')" "$status" > evidence/p10/avx-n256-profile/endpoint-run-81bd07e-b.resource-terminal.txt
