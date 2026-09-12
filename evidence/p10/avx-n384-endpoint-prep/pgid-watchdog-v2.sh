#!/bin/sh
set -eu

if [ "$#" -ne 6 ]; then
    echo "usage: $0 LEADER_PID PROCESS_GROUP STARTTIME CMDLINE_SHA256 DEADLINE_EPOCH LOG" >&2
    exit 64
fi

leader_pid=$1
process_group=$2
expected_starttime=$3
expected_cmdline_sha256=$4
deadline_epoch=$5
log=$6

case "$leader_pid:$process_group:$expected_starttime:$deadline_epoch" in
    *[!0-9:]* | *::* | :* | *:) echo "numeric identity arguments required" >&2; exit 64 ;;
esac
[ "${#expected_cmdline_sha256}" -eq 64 ] || {
    echo "exact lowercase SHA-256 required" >&2
    exit 64
}
case "$expected_cmdline_sha256" in
    *[!0-9a-f]*) echo "exact lowercase SHA-256 required" >&2; exit 64 ;;
esac

identity_matches() {
    [ -r "/proc/$leader_pid/stat" ] && [ -r "/proc/$leader_pid/cmdline" ] || return 1
    stat=$(cat "/proc/$leader_pid/stat" 2>/dev/null) || return 1
    # Strip PID and the parenthesized comm field. The remaining words are
    # proc_pid_stat(5) fields 3 onward, even when comm contains spaces.
    rest=${stat##*) }
    set -- $rest
    [ "$1" != "Z" ] || return 1
    [ "$3" = "$process_group" ] || return 1
    [ "${20}" = "$expected_starttime" ] || return 1
    actual_sha256=$(sha256sum "/proc/$leader_pid/cmdline" 2>/dev/null | awk '{print $1}') || return 1
    [ "$actual_sha256" = "$expected_cmdline_sha256" ]
}

record() {
    printf '%s %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$1" >> "$log"
}

watchdog_pgid=$(ps -o pgid= -p $$ | tr -d ' ')
[ "$watchdog_pgid" != "$process_group" ] || {
    echo "watchdog must run in a different process group" >&2
    exit 64
}
identity_matches || {
    echo "solver identity did not match before monitoring" >&2
    exit 65
}
record "started leader_pid=$leader_pid process_group=$process_group starttime=$expected_starttime cmdline_sha256=$expected_cmdline_sha256 deadline_epoch=$deadline_epoch"

while identity_matches; do
    now=$(date +%s)
    [ "$now" -lt "$deadline_epoch" ] || break
    remaining=$((deadline_epoch - now))
    [ "$remaining" -lt 30 ] && sleep "$remaining" || sleep 30
done

if ! identity_matches; then
    record "solver_exited_or_identity_changed_before_deadline"
    exit 0
fi

record "deadline_reached_sending_TERM"
/bin/kill -TERM -- "-$process_group"
seconds=0
while [ "$seconds" -lt 60 ]; do
    identity_matches || {
        record "solver_exited_after_TERM seconds=$seconds"
        exit 0
    }
    sleep 1
    seconds=$((seconds + 1))
done
identity_matches || {
    record "solver_exited_at_KILL_boundary"
    exit 0
}
record "grace_expired_sending_KILL"
/bin/kill -KILL -- "-$process_group"
