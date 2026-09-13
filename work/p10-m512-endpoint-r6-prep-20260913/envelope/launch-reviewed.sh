#!/bin/sh
# The r6 launcher is deliberately inert unless its exact authorization variable is set.
set -eu

SOURCE=326eeb5cbd5ebe39a7d5f7be77f9acfab8d0db72
BINARY_SHA256=85659a06a1b914d7b64feec20522eb47e876a17c626c3703d4ef59b6cb379b57
SCIENTIFIC_PLAN_SHA256=2be3880204aab5da1819e11ed6abb377e43b814f8ef17869d76463f72a33cf84
PREFLIGHT_SHA256=b085d0678c1576aee4977c413f8072c70a85c1da6249f46cd6bc2fff8b8c1274
WATCHDOG_SHA256=0628e9d65dd65f7738cbaf3bffe720dc357faba1d116d45ec1cdf2c60507ab01
ENVELOPE_SHA256=9f65e80899e78474b489ff969602744e9a44a8baead6e98cf1188dedc55072d9
DEADLINE_EPOCH=1789329174
LATEST_START_EPOCH=1789305197
MEMORY_FLOOR_BYTES=206158430208
DISK_FLOOR_BYTES=137438953472
FIRST_STEP_LIMIT_SECONDS=417
REMAINING_AFTER_FIRST_SECONDS=23560
EXPECTED_BUNDLE=/tmp/nsbu-p10-sulaco-m512-endpoint-r6-20260913

BUNDLE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
pending_pid=
pending_pgid=
pending_starttime=
pending_cmdline_sha256=

pending_matches() {
    [ -n "$pending_pid" ] && [ -r "/proc/$pending_pid/stat" ] && [ -r "/proc/$pending_pid/cmdline" ] || return 1
    pending_stat=$(cat "/proc/$pending_pid/stat" 2>/dev/null) || return 1
    pending_rest=${pending_stat##*) }
    set -- $pending_rest
    [ "$1" != Z ] && [ "$3" = "$pending_pgid" ] && [ "${20}" = "$pending_starttime" ] || return 1
    [ "$(sha256sum "/proc/$pending_pid/cmdline" | awk '{print $1}')" = "$pending_cmdline_sha256" ]
}

cleanup_pending() {
    pending_matches || return 0
    /bin/kill -TERM -- "-$pending_pgid" 2>/dev/null || true
    for _ in $(seq 1 60); do pending_matches || return 0; sleep 1; done
    pending_matches && /bin/kill -KILL -- "-$pending_pgid" 2>/dev/null || true
}

cleanup() {
    status=$?
    trap - EXIT HUP INT TERM
    if [ "$status" -ne 0 ]; then cleanup_pending; fi
    exit "$status"
}

install_cleanup_traps() {
    trap cleanup EXIT
    trap 'exit 129' HUP
    trap 'exit 130' INT
    trap 'exit 143' TERM
}

startup_cleanup_self_test() {
    setsid /bin/sleep 30 &
    pending_pid=$!
    for _ in $(seq 1 100); do
        [ -r "/proc/$pending_pid/stat" ] && [ -r "/proc/$pending_pid/cmdline" ] || { sleep 0.05; continue; }
        pending_stat=$(cat "/proc/$pending_pid/stat")
        pending_rest=${pending_stat##*) }
        set -- $pending_rest
        [ "$1" != Z ] && [ "$3" = "$pending_pid" ] || { sleep 0.05; continue; }
        pending_pgid=$3
        pending_starttime=${20}
        pending_cmdline_sha256=$(sha256sum "/proc/$pending_pid/cmdline" | awk '{print $1}')
        break
    done
    pending_matches || { echo "synthetic startup identity was not captured" >&2; return 1; }
    cleanup_pending
    wait "$pending_pid" 2>/dev/null || true
    if pending_matches; then
        echo "synthetic startup cleanup left owned leader live" >&2
        return 1
    fi
    echo "synthetic startup/watchdog-failure cleanup passed"
}

startup_signal_cleanup_child() {
    pid_file=$1
    setsid /bin/sleep 30 &
    pending_pid=$!
    for _ in $(seq 1 100); do
        [ -r "/proc/$pending_pid/stat" ] && [ -r "/proc/$pending_pid/cmdline" ] || { sleep 0.05; continue; }
        pending_stat=$(cat "/proc/$pending_pid/stat")
        pending_rest=${pending_stat##*) }
        set -- $pending_rest
        [ "$1" != Z ] && [ "$3" = "$pending_pid" ] || { sleep 0.05; continue; }
        pending_pgid=$3
        pending_starttime=${20}
        pending_cmdline_sha256=$(sha256sum "/proc/$pending_pid/cmdline" | awk '{print $1}')
        break
    done
    pending_matches || return 1
    printf '%s\n' "$pending_pid" >"$pid_file"
    /bin/kill -TERM "$$"
    return 1
}

startup_signal_cleanup_self_test() {
    temp_dir=$(mktemp -d "${TMPDIR:-/tmp}/nsbu-r6-signal-cleanup.XXXXXX")
    pid_file=$temp_dir/pid
    if "$0" --self-test-signal-child "$pid_file"; then child_status=0; else child_status=$?; fi
    [ "$child_status" -eq 143 ] || { rm -rf "$temp_dir"; return 1; }
    [ -r "$pid_file" ] || { rm -rf "$temp_dir"; return 1; }
    child_pid=$(cat "$pid_file")
    case "$child_pid" in ''|*[!0-9]*) rm -rf "$temp_dir"; return 1 ;; esac
    for _ in $(seq 1 100); do
        if [ ! -r "/proc/$child_pid/stat" ] || awk '{exit $3 == "Z" ? 0 : 1}' "/proc/$child_pid/stat"; then
            rm -rf "$temp_dir"
            echo "synthetic signal-triggered cleanup passed"
            return 0
        fi
        sleep 0.05
    done
    rm -rf "$temp_dir"
    return 1
}

install_cleanup_traps
if [ "${1:-}" = --self-test-startup-cleanup ]; then startup_cleanup_self_test; exit $?; fi
if [ "${1:-}" = --self-test-signal-child ]; then startup_signal_cleanup_child "${2:?pid file required}"; exit $?; fi
if [ "${1:-}" = --self-test-signal-cleanup ]; then startup_signal_cleanup_self_test; exit $?; fi
[ "${NSBU_LAUNCH_N384_M512_R6:-}" = 1 ] || {
    echo "refused: set NSBU_LAUNCH_N384_M512_R6=1 only after root review" >&2
    exit 64
}
[ "$(hostname)" = sulaco ] || { echo "refused: Sulaco only" >&2; exit 65; }
[ "$BUNDLE" = "$EXPECTED_BUNDLE" ] || { echo "refused: unique r6 stage mismatch" >&2; exit 66; }
BIN=$BUNDLE/p10-avx-n384-m512-piecewise-cadv33
PLAN=$BUNDLE/frozen-plan.json
ENVELOPE=$BUNDLE/frozen-execution-envelope.json
WATCHDOG=$BUNDLE/pgid-watchdog-v3.sh
RUN_ROOT=$BUNDLE/run
OUTPUT=$RUN_ROOT/output
LOG_DIR=$RUN_ROOT/logs

[ ! -e "$RUN_ROOT" ] || { echo "refused: fresh run directory required" >&2; exit 67; }
[ "$(sha256sum "$BIN" | awk '{print $1}')" = "$BINARY_SHA256" ] || exit 67
[ "$(sha256sum "$PLAN" | awk '{print $1}')" = "$SCIENTIFIC_PLAN_SHA256" ] || exit 68
[ "$(sha256sum "$ENVELOPE" | awk '{print $1}')" = "$ENVELOPE_SHA256" ] || exit 69
[ "$(sha256sum "$WATCHDOG" | awk '{print $1}')" = "$WATCHDOG_SHA256" ] || exit 70
now=$(date +%s)
[ "$now" -le "$LATEST_START_EPOCH" ] || { echo "refused: reviewed latest start passed" >&2; exit 71; }
[ "$now" -lt "$DEADLINE_EPOCH" ] || { echo "refused: absolute deadline passed" >&2; exit 72; }
if pgrep -f 'p10-avx.* run ' >/dev/null; then echo "refused: p10 solver exists" >&2; exit 73; fi
available_kib=$(awk '/MemAvailable/{print $2}' /proc/meminfo)
[ "$((available_kib * 1024))" -ge "$MEMORY_FLOOR_BYTES" ] || exit 74
available_disk=$(df -B1 --output=avail "$BUNDLE" | awk 'NR==2{print $1}')
[ "$available_disk" -ge "$DISK_FLOOR_BYTES" ] || exit 75
mkdir -p "$LOG_DIR"
"$BIN" preflight unused >"$LOG_DIR/preflight.stdout"
[ "$(sha256sum "$LOG_DIR/preflight.stdout" | awk '{print $1}')" = "$PREFLIGHT_SHA256" ] || exit 76

timeout_seconds=$((DEADLINE_EPOCH - $(date +%s)))
[ "$timeout_seconds" -gt 0 ] || exit 72
setsid /usr/bin/time -v -o "$LOG_DIR/time.txt" /usr/bin/timeout --foreground --signal=TERM --kill-after=60s "$timeout_seconds" "$BIN" run "$OUTPUT" >"$LOG_DIR/stdout" 2>"$LOG_DIR/stderr" &
time_pid=$!
pending_pid=$time_pid
for _ in $(seq 1 100); do
    [ -r "/proc/$pending_pid/stat" ] && [ -r "/proc/$pending_pid/cmdline" ] || { sleep 0.05; continue; }
    pending_stat=$(cat "/proc/$pending_pid/stat")
    pending_rest=${pending_stat##*) }
    set -- $pending_rest
    [ "$1" != Z ] && [ "$3" = "$pending_pid" ] || { sleep 0.05; continue; }
    pending_pgid=$3
    pending_starttime=${20}
    pending_cmdline_sha256=$(sha256sum "/proc/$pending_pid/cmdline" | awk '{print $1}')
    break
done
pending_matches || { echo "failed to bind setsid/time startup identity" >&2; exit 77; }
for _ in $(seq 1 100); do
    timeout_pid=$(pgrep -P "$time_pid" || true)
    [ -n "$timeout_pid" ] && solver_pid=$(pgrep -P "$timeout_pid" || true) && [ -n "$solver_pid" ] && break
    sleep 0.05
done
[ -n "${solver_pid:-}" ] || exit 78
stat=$(cat "/proc/$solver_pid/stat")
rest=${stat##*) }
set -- $rest
pgid=$3
starttime=${20}
cmdline_sha256=$(sha256sum "/proc/$solver_pid/cmdline" | awk '{print $1}')
[ -n "$pgid" ] && [ -n "$starttime" ] || exit 79
setsid "$WATCHDOG" "$solver_pid" "$pgid" "$starttime" "$cmdline_sha256" "$DEADLINE_EPOCH" "$LOG_DIR/watchdog.log" >"$LOG_DIR/watchdog.stdout" 2>"$LOG_DIR/watchdog.stderr" &
watchdog_pid=$!
watchdog_started="started leader_pid=$solver_pid process_group=$pgid starttime=$starttime cmdline_sha256=$cmdline_sha256 deadline_epoch=$DEADLINE_EPOCH"
for _ in $(seq 1 100); do
    [ -r "/proc/$watchdog_pid/stat" ] && grep -Fq "$watchdog_started" "$LOG_DIR/watchdog.log" 2>/dev/null && break
    sleep 0.05
done
[ -r "/proc/$watchdog_pid/stat" ] && grep -Fq "$watchdog_started" "$LOG_DIR/watchdog.log" 2>/dev/null || exit 87
printf 'source=%s\ntimeout_seconds=%s\nsolver_pid=%s\nprocess_group=%s\nstarttime=%s\ncmdline_sha256=%s\nwatchdog_pid=%s\ndeadline_epoch=%s\n' "$SOURCE" "$timeout_seconds" "$solver_pid" "$pgid" "$starttime" "$cmdline_sha256" "$watchdog_pid" "$DEADLINE_EPOCH" >"$LOG_DIR/launch-identity.txt"
for waited in $(seq 0 5 1200); do
    grep -q '^attempt=1 clock=64 ' "$LOG_DIR/stdout" && break
    kill -0 "$solver_pid" 2>/dev/null || exit 79
    kill -0 "$watchdog_pid" 2>/dev/null || exit 80
    sleep 5
done
first=$(grep '^attempt=1 clock=64 ' "$LOG_DIR/stdout" | head -n 1)
[ -n "$first" ] || exit 81
printf '%s\n' "$first" | grep -q 'rhs_timed_calls=12 ' || exit 82
printf '%s\n' "$first" | grep -q 'cache_hit_miss=\[7, 5\] ' || exit 83
printf '%s\n' "$first" | grep -q 'steady_allocations=0$' || exit 84
first_attempt=$OUTPUT/step-001-clock-0064/attempt.json
[ -r "$first_attempt" ] || exit 88
expected_identity=$(sed -n 's/.* profile_identity=\(.*\) archive_profile=.*/\1/p' "$LOG_DIR/preflight.stdout")
[ -n "$expected_identity" ] && grep -Fq "\"identity\": \"$expected_identity\"" "$first_attempt" || exit 89
first_seconds=$(printf '%s\n' "$first" | sed -n 's/.*integration_seconds=\([0-9.]*\).*/\1/p')
awk -v actual="$first_seconds" -v limit="$FIRST_STEP_LIMIT_SECONDS" 'BEGIN{exit !(actual ~ /^[0-9]+([.][0-9]+)?$/ && actual + 0 <= limit)}' || exit 85
remaining=$((DEADLINE_EPOCH - $(date +%s)))
[ "$remaining" -ge "$REMAINING_AFTER_FIRST_SECONDS" ] || exit 90
trap - EXIT HUP INT TERM
echo "first-step gate passed; watchdog owns process group $pgid through absolute deadline $DEADLINE_EPOCH"
