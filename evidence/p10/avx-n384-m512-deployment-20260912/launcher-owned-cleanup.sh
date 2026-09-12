#!/bin/sh
# Sourced by launch-reviewed.sh after this file's SHA-256 is verified.

: "${LAUNCH_STARTED:=0}"
: "${LAUNCH_HANDOFF:=0}"
: "${SPAWN_PENDING:=0}"
: "${SPAWN_PID:=}"
: "${SPAWN_PARENT_PID:=}"
: "${SPAWN_STARTTIME:=}"
: "${TIME_PID:=}"
: "${TIME_STARTTIME:=}"
: "${TIME_CMDLINE_SHA256:=}"
: "${SOLVER_PID:=}"
: "${PROCESS_GROUP:=}"
: "${STARTTIME:=}"
: "${CMDLINE_SHA256:=}"
: "${CLEANUP_GRACE_SECONDS:=60}"

owned_member_matches() {
    owned_pid=$1
    owned_group=$2
    owned_starttime=$3
    owned_cmdline_sha256=$4
    [ -n "$owned_pid" ] && [ -r "/proc/$owned_pid/stat" ] && [ -r "/proc/$owned_pid/cmdline" ] || return 1
    owned_stat=$(cat "/proc/$owned_pid/stat" 2>/dev/null) || return 1
    owned_rest=${owned_stat##*) }
    set -- $owned_rest
    [ "$1" != Z ] || return 1
    [ "$3" = "$owned_group" ] || return 1
    [ "${20}" = "$owned_starttime" ] || return 1
    owned_actual_cmdline=$(sha256sum "/proc/$owned_pid/cmdline" 2>/dev/null | awk '{print $1}') || return 1
    [ "$owned_actual_cmdline" = "$owned_cmdline_sha256" ]
}

owned_group_matches() {
    if [ -n "$SOLVER_PID" ] && owned_member_matches "$SOLVER_PID" "$PROCESS_GROUP" "$STARTTIME" "$CMDLINE_SHA256"; then
        return 0
    fi
    if [ -n "$TIME_PID" ] && owned_member_matches "$TIME_PID" "$PROCESS_GROUP" "$TIME_STARTTIME" "$TIME_CMDLINE_SHA256"; then
        return 0
    fi
    return 1
}

pending_spawn_identity() {
    [ "$SPAWN_PENDING" -eq 1 ] && [ -n "$SPAWN_PID" ] && [ -r "/proc/$SPAWN_PID/stat" ] || return 1
    pending_stat=$(cat "/proc/$SPAWN_PID/stat" 2>/dev/null) || return 1
    pending_rest=${pending_stat##*) }
    set -- $pending_rest
    [ "$1" != Z ] || return 1
    [ "$2" = "$SPAWN_PARENT_PID" ] || return 1
    if [ -n "$SPAWN_STARTTIME" ]; then
        [ "${20}" = "$SPAWN_STARTTIME" ] || return 1
    fi
    PENDING_PROCESS_GROUP=$3
}

cleanup_pending_spawn() {
    pending_spawn_identity || return 0
    if [ "$PENDING_PROCESS_GROUP" = "$SPAWN_PID" ]; then
        /bin/kill -TERM -- "-$PENDING_PROCESS_GROUP"
    else
        /bin/kill -TERM "$SPAWN_PID"
    fi
    cleanup_seconds=0
    while [ "$cleanup_seconds" -lt "$CLEANUP_GRACE_SECONDS" ]; do
        pending_spawn_identity || return 0
        sleep 1
        cleanup_seconds=$((cleanup_seconds + 1))
    done
    if pending_spawn_identity; then
        if [ "$PENDING_PROCESS_GROUP" = "$SPAWN_PID" ]; then
            /bin/kill -KILL -- "-$PENDING_PROCESS_GROUP"
        else
            /bin/kill -KILL "$SPAWN_PID"
        fi
    fi
    return 0
}

cleanup_owned_group() {
    [ "$LAUNCH_STARTED" -eq 1 ] || return 0
    if [ "$SPAWN_PENDING" -eq 1 ]; then
        cleanup_pending_spawn
        return 0
    fi
    owned_group_matches || return 0
    /bin/kill -TERM -- "-$PROCESS_GROUP"
    cleanup_seconds=0
    while [ "$cleanup_seconds" -lt "$CLEANUP_GRACE_SECONDS" ]; do
        owned_group_matches || return 0
        sleep 1
        cleanup_seconds=$((cleanup_seconds + 1))
    done
    if owned_group_matches; then
        /bin/kill -KILL -- "-$PROCESS_GROUP"
    fi
    return 0
}

launcher_exit_trap() {
    launcher_status=$?
    trap - EXIT HUP INT TERM
    if [ "$launcher_status" -ne 0 ] && [ "$LAUNCH_HANDOFF" -ne 1 ]; then
        cleanup_owned_group
    fi
    exit "$launcher_status"
}

install_owned_cleanup_traps() {
    trap launcher_exit_trap EXIT
    trap 'exit 129' HUP
    trap 'exit 130' INT
    trap 'exit 143' TERM
}
