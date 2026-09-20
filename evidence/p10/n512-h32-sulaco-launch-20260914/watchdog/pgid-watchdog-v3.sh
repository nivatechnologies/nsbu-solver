#!/bin/sh
# r5 repaired pgid watchdog. In addition to the reviewed r3 attachment rules:
# * LEADER EXIT NO LONGER ENDS DESCENDANT ESCALATION: when the guarded leader
#   is gone (exited/dead) the watchdog keeps TERMs/KILLs every /proc member
#   of the guarded process group, VERIFIES group disappearance after KILL and
#   preserves any adverse outcome with a non-zero exit.
# * EVERY SIGNAL IS IDENTITY-BOUND: before each individual TERM and each
#   individual KILL the member's immutable starttime+cmdline identity is
#   re-read and compared against the frozen enumeration; a member whose
#   identity drifted (live process, same PID/starttime, changed cmdline —
#   the PID-recycle/exec pattern) is NEVER signalled; the refusal and the
#   surviving members are recorded and the watchdog exits non-zero.
# * IDENTITY DOUBT NEVER SIGNALS: an identity mismatch at the deadline or a
#   live-but-changed leader (starttime/cmdline/process_group drift or an
#   unreadable /proc) means the PID may have been recycled — the watchdog
#   refuses to signal ANYTHING and exits 65, making the launch fail
#   truthfully. Only a leader that is truly gone (proc missing or zombie)
#   keeps the group drainable: its group members are still our descendants.
set -eu

[ "$#" -eq 6 ] || { echo "usage: $0 LEADER_PID PROCESS_GROUP STARTTIME CMDLINE_SHA256 DEADLINE_EPOCH LOG" >&2; exit 64; }
leader_pid=$1
process_group=$2
expected_starttime=$3
expected_cmdline_sha256=$4
deadline_epoch=$5
log=$6
poll_seconds=${WATCHDOG_POLL_SECONDS:-30}
grace_seconds=${WATCHDOG_GRACE_SECONDS:-60}
case "$leader_pid:$process_group:$expected_starttime:$deadline_epoch:$poll_seconds:$grace_seconds" in
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

# True when the leader is GONE for certain (proc vanished or zombie): only
# then may the group be escalated without the leader's own identity check.
leader_confirmed_gone() {
    case "$1" in
        proc_missing|state) return 0 ;;
        *) return 1 ;;
    esac
}

# Live, non-zombie members currently in the guarded process group as
# "pid:starttime:cmdline_sha256" rows (frozen immutable identity rows).
member_rows() {
    for d in /proc/[0-9]*; do
        p=${d##*/}
        s=$(cat "$d/stat" 2>/dev/null) || continue
        r=${s##*) }
        set -- $r
        [ "$#" -ge 20 ] || continue
        [ "$1" = Z ] && continue
        [ "$3" = "$process_group" ] || continue
        h=$(sha256sum "$d/cmdline" 2>/dev/null | awk '{print $1}') || h=unreadable
        printf '%s:%s:%s\n' "$p" "${20}" "$h"
    done
}

# Current "starttime cmdline_sha" identity of one PID (empty + false when
# gone, zombie or unreadable).
identity_row() {
    p=$1
    [ -r "/proc/$p/stat" ] || return 1
    s=$(cat "/proc/$p/stat" 2>/dev/null) || return 1
    r=${s##*) }
    set -- $r
    [ "$#" -ge 20 ] || return 1
    [ "$1" = Z ] && return 1
    h=$(sha256sum "/proc/$p/cmdline" 2>/dev/null | awk '{print $1}') || return 1
    printf '%s %s' "${20}" "$h"
}

member_pids() {
    rows=$1
    out=""
    for row in $rows; do
        out="$out ${row%%:*}"
    done
    echo $out
}

member_empty() {
    [ -z "$(member_rows)" ]
}

# Frozen "starttime sha" identity of a PID from the global frozen_rows, or
# empty when the PID was never frozen (an unfrozen member is NEVER signalled).
frozen_identity_of() {
    want=$1
    for frow in $frozen_rows; do
        [ "${frow%%:*}" = "$want" ] || continue
        rest=${frow#*:}
        printf '%s %s' "${rest%%:*}" "${rest#*:}"
        return 0
    done
    return 1
}

# Signal every enumerated member whose identity STILL matches the FROZEN
# identity (never the re-enumerated one: a member that exec-changed between
# the freeze and this wave looks identical to itself and must not mask the
# drift). Unfrozen, drifted or unreadable members are refused and never
# signalled. $1 = rows, $2 = "TERM" or "KILL", $3 = reason. Sets globals
# wave_sent and drift_refused in THIS shell (no subshell, so the refusal is
# observable).
signal_wave() {
    rows=$1
    signum=$2
    reason=$3
    wave_sent=0
    for row in $rows; do
        pid=${row%%:*}
        want=$(frozen_identity_of "$pid") || want=""
        if [ -z "$want" ]; then
            record "member_${signum}_refused_unfrozen_identity pid=$pid reason=$reason"
            drift_refused=1
            continue
        fi
        st=${want% *}
        sha=${want#* }
        cur=$(identity_row "$pid" 2>/dev/null) || cur=""
        if [ -z "$cur" ]; then
            record "member_${signum}_skipped_gone_or_unreadable pid=$pid reason=$reason"
            continue
        fi
        if [ "$cur" != "$st $sha" ] || [ "$st" = "unreadable" ]; then
            record "member_${signum}_refused_identity_drift pid=$pid frozen=$st:$sha current=$cur reason=$reason"
            drift_refused=1
            continue
        fi
        /bin/kill -"$signum" -- "$pid" 2>/dev/null || true
        wave_sent=$((wave_sent + 1))
    done
}

# TERM the group members, wait the pinned grace, KILL survivors, VERIFY
# disappearance — with per-signal identity revalidation above. Adverse
# outcomes (survivors after KILL, drift refusals) are recorded and returned
# non-zero.
escalate_group() {
    reason=$1
    rows=$(member_rows)
    if [ -z "$rows" ]; then
        record "group_already_gone reason=$reason"
        return 0
    fi
    frozen_rows=$rows
    drift_refused=0
    signal_wave "$frozen_rows" TERM "$reason"
    record "group_escalation_TERM reason=$reason members=$(member_pids "$frozen_rows") sent=$wave_sent"
    seconds=0
    while [ "$seconds" -lt "$grace_seconds" ]; do
        rows=$(member_rows)
        if [ -z "$rows" ]; then
            record "group_cleared_after_TERM reason=$reason seconds=$seconds"
            [ "$drift_refused" -eq 1 ] && record "escalation_had_drift_refusals reason=$reason"
            return 0
        fi
        # Late joiners (born after the freeze): freeze + identity-bound TERM.
        new_rows=""
        for row in $rows; do
            pid=${row%%:*}
            case " $(member_pids "$frozen_rows") " in
                *" $pid "*) : ;;
                *) new_rows="$new_rows $row" ;;
            esac
        done
        if [ -n "$new_rows" ]; then
            signal_wave "$new_rows" TERM "$reason"
            record "group_escalation_TERM_late_joiners reason=$reason members=$(member_pids "$new_rows") sent=$wave_sent"
            frozen_rows="$frozen_rows
$new_rows"
        fi
        sleep 1
        seconds=$((seconds + 1))
    done
    rows=$(member_rows)
    signal_wave "$rows" KILL "$reason"
    record "grace_expired_sending_KILL reason=$reason members=$(member_pids "$rows") sent=$wave_sent"
    seconds=0
    kill_deadline=$grace_seconds
    while [ "$seconds" -lt "$kill_deadline" ]; do
        if member_empty; then
            record "group_cleared_after_KILL reason=$reason seconds=$seconds"
            [ "$drift_refused" -eq 1 ] && record "escalation_had_drift_refusals reason=$reason"
            return 0
        fi
        sleep 1
        seconds=$((seconds + 1))
    done
    record "group_survivors_after_KILL reason=$reason members=$(member_pids "$(member_rows)") drift_refused=$drift_refused"
    return 70
}

watchdog_pgid=$(ps -o pgid= -p $$ | tr -d ' ')
[ "$watchdog_pgid" != "$process_group" ] || exit 64
record "started leader_pid=$leader_pid process_group=$process_group starttime=$expected_starttime cmdline_sha256=$expected_cmdline_sha256 deadline_epoch=$deadline_epoch"
if ! sample_identity; then
    if ! leader_confirmed_gone "$mismatch_component"; then
        # Live-but-changed leader at attachment (recycle/exec suspect):
        # NEVER signal anything; make the launch fail truthfully.
        record "attachment_identity_mismatch $(diagnostic)"
        exit 65
    fi
    # Leader already gone when we attached: nothing is running to protect;
    # drain the (possibly orphaned) group now with identity-bound signals.
    record "attachment_leader_already_gone $(diagnostic)"
    if ! escalate_group "leader_gone_at_attachment"; then
        record "watchdog_adverse attach_escalation_failed"
        exit 70
    fi
    record "watchdog_complete reason=leader_gone_at_attachment_group_cleared"
    exit 0
fi

while :; do
    now=$(date +%s)
    [ "$now" -lt "$deadline_epoch" ] || break
    remaining=$((deadline_epoch - now))
    sleep_for=$poll_seconds
    [ "$remaining" -lt "$sleep_for" ] && sleep_for=$remaining
    sleep "$sleep_for"
    [ "$(date +%s)" -lt "$deadline_epoch" ] || break
    if ! sample_identity; then
        first=$mismatch_component
        first_diag=$(diagnostic)
        record "identity_sample_failed component=$first $first_diag"
        sleep 1
        if sample_identity; then record "identity_sample_recovered prior_component=$first"; continue; fi
        # Stable identity loss confirmed by two consecutive stable samples.
        if ! leader_confirmed_gone "$first"; then
            # Possible PID recycle, unreadable /proc, group change or an
            # exec-changed cmdline on a LIVE pid: identity doubt. Signals are
            # refused entirely; the launch must fail truthfully.
            record "identity_mismatch_confirmed_refusing_signals first_{$first_diag} second_{$(diagnostic)}"
            exit 65
        fi
        # Leader gone (proc_missing / zombie state): descendants still need
        # escalation; each member signal is identity-bound inside escalate.
        if ! escalate_group "leader_identity_lost_$first"; then
            record "watchdog_adverse leader_gone_escalation_failed"
            exit 70
        fi
        record "watchdog_complete leader_gone_escalation reason=leader_identity_lost_$first"
        exit 0
    fi
done

now=$(date +%s)
[ "$now" -ge "$deadline_epoch" ] || { record "internal_refusal_TERM_before_deadline now=$now"; exit 70; }
if ! sample_identity; then
    if ! leader_confirmed_gone "$mismatch_component"; then
        # Deadline reached while the leader identity is in doubt: NEVER
        # signal, refuse, and make the launch fail truthfully.
        record "deadline_identity_refusal_refusing_signals $(diagnostic)"
        exit 65
    fi
    record "identity_mismatch_at_deadline_leader_gone $(diagnostic)"
fi
record "deadline_reached_sending_TERM now=$now"
if ! escalate_group "absolute_deadline"; then
    record "watchdog_adverse deadline_escalation_failed"
    exit 70
fi
record "watchdog_complete reason=absolute_deadline_group_cleared"
exit 0
