#!/bin/sh
set -eu

solver_pid=418511
process_group=418509
expected_starttime=3218683
expected_command='target/avx-scheduled-endpoint-v4/release/p10-avx-scheduled-endpoint run evidence/p10/avx-scheduled-endpoint/run-92effa6 '
deadline_epoch=1789259400
terminal=evidence/p10/avx-scheduled-endpoint/run-92effa6.watchdog-terminal.txt

same_solver() {
    [ -r "/proc/$solver_pid/stat" ] || return 1
    [ "$(awk '{ print $22 }' "/proc/$solver_pid/stat")" = "$expected_starttime" ] || return 1
    [ "$(tr '\000' ' ' < "/proc/$solver_pid/cmdline")" = "$expected_command" ]
}

while same_solver && [ "$(date -u +%s)" -lt "$deadline_epoch" ]; do
    sleep 30
done

if ! same_solver; then
    printf '%s,status=solver_exited_or_identity_changed,no_signal_sent\n' "$(date -u +'%Y-%m-%dT%H:%M:%SZ')" > "$terminal"
    exit 0
fi

kill -TERM -- "-$process_group"
printf '%s,status=deadline_term_sent,pgid=%s\n' "$(date -u +'%Y-%m-%dT%H:%M:%SZ')" "$process_group" > "$terminal"
sleep 60
if same_solver; then
    kill -KILL -- "-$process_group"
    printf '%s,status=deadline_kill_sent,pgid=%s\n' "$(date -u +'%Y-%m-%dT%H:%M:%SZ')" "$process_group" >> "$terminal"
fi
