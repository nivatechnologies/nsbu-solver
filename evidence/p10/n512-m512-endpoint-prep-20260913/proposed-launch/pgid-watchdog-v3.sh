#!/bin/sh
set -eu

[ "$#" -eq 6 ] || { echo "usage: $0 LEADER_PID PROCESS_GROUP STARTTIME CMDLINE_SHA256 DEADLINE_EPOCH LOG" >&2; exit 64; }
leader_pid=$1
process_group=$2
expected_starttime=$3
expected_cmdline_sha256=$4
deadline_epoch=$5
log=$6
poll_seconds=${WATCHDOG_POLL_SECONDS:-30}
case "$leader_pid:$process_group:$expected_starttime:$deadline_epoch:$poll_seconds" in
    *[!0-9:]* | *::* | :* | *:) echo "numeric identity arguments required" >&2; exit 64 ;;
esac
[ "${#expected_cmdline_sha256}" -eq 64 ] || exit 64
case "$expected_cmdline_sha256" in *[!0-9a-f]*) exit 64 ;; esac

record() { printf '%s %s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" "$1" >>"$log"; }

sample_identity() {
    observed_state=missing
    observed_process_group=missing
    observed_starttime=missing
    observed_cmdline_sha256=missing
    mismatch_component=proc_missing
    [ -r "/proc/$leader_pid/stat" ] && [ -r "/proc/$leader_pid/cmdline" ] || return 1
    observed_stat=$(cat "/proc/$leader_pid/stat" 2>/dev/null) || { mismatch_component=stat_read; return 1; }
    observed_rest=${observed_stat##*) }
    set -- $observed_rest
    [ "$#" -ge 20 ] || { mismatch_component=stat_parse; return 1; }
    observed_state=$1
    observed_process_group=$3
    observed_starttime=${20}
    observed_cmdline_sha256=$(sha256sum "/proc/$leader_pid/cmdline" 2>/dev/null | awk '{print $1}') || { mismatch_component=cmdline_read; return 1; }
    [ "$observed_state" != Z ] || { mismatch_component=state; return 1; }
    [ "$observed_process_group" = "$process_group" ] || { mismatch_component=process_group; return 1; }
    [ "$observed_starttime" = "$expected_starttime" ] || { mismatch_component=starttime; return 1; }
    [ "$observed_cmdline_sha256" = "$expected_cmdline_sha256" ] || { mismatch_component=cmdline_sha256; return 1; }
    mismatch_component=none
}

diagnostic() {
    printf 'component=%s expected_state=non_zombie observed_state=%s expected_process_group=%s observed_process_group=%s expected_starttime=%s observed_starttime=%s expected_cmdline_sha256=%s observed_cmdline_sha256=%s' \
        "$mismatch_component" "$observed_state" "$process_group" "$observed_process_group" "$expected_starttime" "$observed_starttime" "$expected_cmdline_sha256" "$observed_cmdline_sha256"
}

watchdog_pgid=$(ps -o pgid= -p $$ | tr -d ' ')
[ "$watchdog_pgid" != "$process_group" ] || exit 64
sample_identity || { record "attachment_identity_mismatch $(diagnostic)"; exit 65; }
record "started leader_pid=$leader_pid process_group=$process_group starttime=$expected_starttime cmdline_sha256=$expected_cmdline_sha256 deadline_epoch=$deadline_epoch"

while :; do
    now=$(date +%s)
    [ "$now" -lt "$deadline_epoch" ] || break
    remaining=$((deadline_epoch - now))
    sleep_for=$poll_seconds
    [ "$remaining" -lt "$sleep_for" ] && sleep_for=$remaining
    sleep "$sleep_for"
    [ "$(date +%s)" -lt "$deadline_epoch" ] || break
    if ! sample_identity; then
        first=$(diagnostic)
        record "identity_sample_failed $first"
        sleep 1
        if sample_identity; then record "identity_sample_recovered prior_$first"; continue; fi
        record "identity_mismatch_confirmed first_{$first} second_{$(diagnostic)}"
        exit 0
    fi
done

now=$(date +%s)
[ "$now" -ge "$deadline_epoch" ] || { record "internal_refusal_TERM_before_deadline now=$now"; exit 70; }
sample_identity || { record "identity_mismatch_at_deadline $(diagnostic)"; exit 0; }
record "deadline_reached_sending_TERM now=$now"
/bin/kill -TERM -- "-$process_group"
seconds=0
while [ "$seconds" -lt 60 ]; do
    sample_identity || { record "solver_exited_after_TERM seconds=$seconds $(diagnostic)"; exit 0; }
    sleep 1
    seconds=$((seconds + 1))
done
sample_identity || { record "solver_exited_at_KILL_boundary $(diagnostic)"; exit 0; }
record "grace_expired_sending_KILL"
/bin/kill -KILL -- "-$process_group"
