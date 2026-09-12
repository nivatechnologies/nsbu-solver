#!/bin/sh
set -eu

HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
. "$HERE/launcher-owned-cleanup.sh"
CLEANUP_GRACE_SECONDS=2

capture_identity() {
    capture_pid=$1
    capture_stat=$(cat "/proc/$capture_pid/stat")
    capture_rest=${capture_stat##*) }
    set -- $capture_rest
    PROCESS_GROUP=$3
    TIME_STARTTIME=${20}
    TIME_CMDLINE_SHA256=$(sha256sum "/proc/$capture_pid/cmdline" | awk '{print $1}')
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

echo "capture_exit80_cleanup=passed child_group_exit81_cleanup=passed attachment_exit96_cleanup=passed"
