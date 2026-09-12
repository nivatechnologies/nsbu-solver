#!/bin/sh
set -eu

HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
. "$HERE/launcher-owned-cleanup.sh"
CLEANUP_GRACE_SECONDS=2

capture_identity() {
    capture_pid=$1
    for _ in $(seq 1 100); do
        capture_stat=$(cat "/proc/$capture_pid/stat" 2>/dev/null || true)
        capture_rest=${capture_stat##*) }
        set -- $capture_rest
        if [ "$#" -ge 20 ] && [ "$1" != Z ] && [ "$3" = "$capture_pid" ]; then
            capture_argv0=$(tr '\000' '\n' <"/proc/$capture_pid/cmdline" | head -n 1)
            if [ "$capture_argv0" = sh ]; then
                PROCESS_GROUP=$3
                TIME_STARTTIME=${20}
                TIME_CMDLINE_SHA256=$(sha256sum "/proc/$capture_pid/cmdline" | awk '{print $1}')
                return 0
            fi
        fi
        sleep 0.02
    done
    return 1
}

assert_gone() {
    gone_pid=$1
    for _ in $(seq 1 50); do
        kill -0 "$gone_pid" 2>/dev/null || return 0
        sleep 0.02
    done
    echo "dummy PID $gone_pid survived cleanup" >&2
    return 1
}

capture_solver_child() {
    SOLVER_PID=
    for _ in $(seq 1 50); do
        SOLVER_PID=$(pgrep -P "$TIME_PID" || true)
        [ -n "$SOLVER_PID" ] && break
        sleep 0.02
    done
    [ -n "$SOLVER_PID" ]
    solver_stat=$(cat "/proc/$SOLVER_PID/stat")
    solver_rest=${solver_stat##*) }
    set -- $solver_rest
    STARTTIME=${20}
    CMDLINE_SHA256=$(sha256sum "/proc/$SOLVER_PID/cmdline" | awk '{print $1}')
}

start_wrapper_group() {
    setsid sh -c 'trap "exit 0" TERM; sh -c '\''trap "exit 0" TERM; while :; do sleep 1; done'\'' & wait' &
    TIME_PID=$!
    capture_identity "$TIME_PID"
    capture_solver_child
    LAUNCH_STARTED=1
}

reset_pending() {
    LAUNCH_STARTED=1
    LAUNCH_HANDOFF=0
    SPAWN_PID=$1
    SPAWN_PARENT_PID=$$
    SPAWN_STARTTIME=
    SPAWN_PENDING=1
    TIME_PID=
    SOLVER_PID=
    PROCESS_GROUP=
    STARTTIME=
    CMDLINE_SHA256=
}

# Before setsid has changed the group, cleanup is restricted to the direct child.
sh -c 'sleep 5; exec setsid sh -c '\''while :; do sleep 1; done'\''' &
pending_pid=$!
reset_pending "$pending_pid"
trap_status=0
(install_owned_cleanup_traps; exit 98) || trap_status=$?
[ "$trap_status" -eq 98 ]
wait "$pending_pid" || true
assert_gone "$pending_pid"

# Immediately after an actual setsid spawn, the same trap handles either side of the PGID race.
setsid sh -c 'while :; do sleep 1; done' &
pending_pid=$!
reset_pending "$pending_pid"
trap_status=0
(install_owned_cleanup_traps; exit 99) || trap_status=$?
[ "$trap_status" -eq 99 ]
wait "$pending_pid" || true
assert_gone "$pending_pid"
SPAWN_PENDING=0

# Capture failure: only the exact setsid leader identity is available.
setsid sh -c 'trap "exit 0" TERM; while :; do sleep 1; done' &
TIME_PID=$!
capture_identity "$TIME_PID"
LAUNCH_STARTED=1
SOLVER_PID=
STARTTIME=
CMDLINE_SHA256=
trap_status=0
(install_owned_cleanup_traps; exit 80) || trap_status=$?
[ "$trap_status" -eq 80 ]
wait "$TIME_PID" || true
assert_gone "$TIME_PID"

# Child group-validation failure: wrapper and child identities are known.
start_wrapper_group
trap_status=0
(install_owned_cleanup_traps; exit 81) || trap_status=$?
[ "$trap_status" -eq 81 ]
wait "$TIME_PID" || true
assert_gone "$TIME_PID"
assert_gone "$SOLVER_PID"

# Watchdog attachment failure: the same exact ownership proof must clean up.
start_wrapper_group
trap_status=0
(install_owned_cleanup_traps; exit 96) || trap_status=$?
[ "$trap_status" -eq 96 ]
wait "$TIME_PID" || true
assert_gone "$TIME_PID"
assert_gone "$SOLVER_PID"

echo "pre_setsid_cleanup=passed post_spawn_race_cleanup=passed capture_exit80_cleanup=passed child_group_exit81_cleanup=passed attachment_exit96_cleanup=passed"
