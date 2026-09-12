#!/bin/sh
set -eu

SOURCE=326eeb5cbd5ebe39a7d5f7be77f9acfab8d0db72
BINARY_SHA256=85659a06a1b914d7b64feec20522eb47e876a17c626c3703d4ef59b6cb379b57
WATCHDOG_SHA256=4b65e13b74bd7a32237044b5467d4f223c8dc3e6e35d6e8d3498e1938fa53e4b
PLAN_SHA256=2be3880204aab5da1819e11ed6abb377e43b814f8ef17869d76463f72a33cf84
PREFLIGHT_SHA256=b085d0678c1576aee4977c413f8072c70a85c1da6249f46cd6bc2fff8b8c1274
LATEST_START_EPOCH=1789260352
DEADLINE_EPOCH=1789282800
HARD_BLOCK_EPOCH=1789283807
MEMORY_FLOOR_BYTES=206158430208
DISK_FLOOR_BYTES=137438953472
FIRST_STEP_LIMIT_SECONDS=399
REMAINING_AFTER_FIRST_SECONDS=21359
PREDECESSOR=/tmp/nsbu-p10-sulaco-20260912T1420Z/n384-piecewise-cadv33-aed49b7/endpoint

[ "${NSBU_LAUNCH_N384_M512:-}" = 1 ] || {
    echo "root must set NSBU_LAUNCH_N384_M512=1 for this exact reviewed launch" >&2
    exit 64
}
[ "$(hostname)" = sulaco ] || { echo "host must be sulaco" >&2; exit 65; }

BUNDLE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
BIN=$BUNDLE/p10-avx-n384-m512-piecewise-cadv33
WATCHDOG=$BUNDLE/pgid-watchdog-v2.sh
PLAN=$BUNDLE/frozen-plan.json
RUN_ROOT=$BUNDLE/run
OUTPUT=$RUN_ROOT/output
LOG_DIR=$RUN_ROOT/logs

[ ! -e "$RUN_ROOT" ] || { echo "fresh run directory required" >&2; exit 66; }
[ "$(sha256sum "$BIN" | awk '{print $1}')" = "$BINARY_SHA256" ] || exit 67
[ "$(sha256sum "$WATCHDOG" | awk '{print $1}')" = "$WATCHDOG_SHA256" ] || exit 68
[ "$(sha256sum "$PLAN" | awk '{print $1}')" = "$PLAN_SHA256" ] || exit 69
now=$(date +%s)
[ "$now" -le "$LATEST_START_EPOCH" ] || { echo "latest reviewed start passed" >&2; exit 70; }
[ "$now" -lt "$DEADLINE_EPOCH" ] && [ "$DEADLINE_EPOCH" -lt "$HARD_BLOCK_EPOCH" ] || exit 71

[ -r "$PREDECESSOR/run.stdout" ] || { echo "predecessor record missing" >&2; exit 72; }
[ "$(awk '/^attempt=/{n++} END{print n+0}' "$PREDECESSOR/run.stdout")" -eq 48 ] || {
    echo "predecessor has not completed 48 attempts" >&2
    exit 73
}
awk '/^attempt=/{line=$0} END{print line}' "$PREDECESSOR/run.stdout" | grep -q '^attempt=48 clock=4096 ' || exit 74
grep -q '^terminal endpoint_complete_qualification_pending clock=4096$' "$PREDECESSOR/run.stdout" || exit 92
[ -r "$PREDECESSOR/run.time" ] && grep -q '^[[:space:]]*Exit status: 0$' "$PREDECESSOR/run.time" || exit 93
if pgrep -f 'p10-avx.* run ' >/dev/null; then
    echo "whole Sulaco host must be free of p10-avx solvers" >&2
    exit 75
fi

mkdir -p "$LOG_DIR"
awk '/MemTotal|MemAvailable|SwapFree/ {print}' /proc/meminfo >"$LOG_DIR/memory-before.txt"
df -B1 "$BUNDLE" >"$LOG_DIR/disk-before.txt"
available_kib=$(awk '/MemAvailable/{print $2}' /proc/meminfo)
[ "$((available_kib * 1024))" -ge "$MEMORY_FLOOR_BYTES" ] || exit 76
available_disk=$(df -B1 --output=avail "$BUNDLE" | awk 'NR==2{print $1}')
[ "$available_disk" -ge "$DISK_FLOOR_BYTES" ] || exit 77

"$BIN" preflight unused >"$LOG_DIR/preflight.stdout"
[ "$(sha256sum "$LOG_DIR/preflight.stdout" | awk '{print $1}')" = "$PREFLIGHT_SHA256" ] || exit 78
[ ! -e "$OUTPUT" ] || exit 79

setsid /usr/bin/time -v -o "$LOG_DIR/time.txt" "$BIN" run "$OUTPUT" >"$LOG_DIR/stdout" 2>"$LOG_DIR/stderr" &
TIME_PID=$!
SOLVER_PID=
for _ in $(seq 1 100); do
    children=$(pgrep -P "$TIME_PID" || true)
    if [ "$(printf '%s\n' "$children" | sed '/^$/d' | wc -l)" -eq 1 ]; then
        SOLVER_PID=$children
        break
    fi
    sleep 0.05
done
[ -n "$SOLVER_PID" ] || exit 80
proc_stat=$(cat "/proc/$SOLVER_PID/stat")
proc_rest=${proc_stat##*) }
set -- $proc_rest
[ "$3" = "$TIME_PID" ] || exit 81
PROCESS_GROUP=$3
STARTTIME=${20}
CMDLINE_SHA256=$(sha256sum "/proc/$SOLVER_PID/cmdline" | awk '{print $1}')
setsid "$WATCHDOG" "$SOLVER_PID" "$PROCESS_GROUP" "$STARTTIME" "$CMDLINE_SHA256" "$DEADLINE_EPOCH" "$LOG_DIR/watchdog.log" >"$LOG_DIR/watchdog.stdout" 2>"$LOG_DIR/watchdog.stderr" &
WATCHDOG_PID=$!

solver_identity_matches() {
    [ -r "/proc/$SOLVER_PID/stat" ] && [ -r "/proc/$SOLVER_PID/cmdline" ] || return 1
    identity_stat=$(cat "/proc/$SOLVER_PID/stat" 2>/dev/null) || return 1
    identity_rest=${identity_stat##*) }
    set -- $identity_rest
    [ "$1" != Z ] || return 1
    [ "$3" = "$PROCESS_GROUP" ] || return 1
    [ "${20}" = "$STARTTIME" ] || return 1
    identity_cmdline=$(sha256sum "/proc/$SOLVER_PID/cmdline" 2>/dev/null | awk '{print $1}') || return 1
    [ "$identity_cmdline" = "$CMDLINE_SHA256" ]
}

stop_solver() {
    solver_identity_matches || return 0
    /bin/kill -TERM -- "-$PROCESS_GROUP"
    stop_seconds=0
    while [ "$stop_seconds" -lt 60 ]; do
        solver_identity_matches || return 0
        sleep 1
        stop_seconds=$((stop_seconds + 1))
    done
    solver_identity_matches && /bin/kill -KILL -- "-$PROCESS_GROUP"
    return 0
}

{
    echo "source=$SOURCE"
    echo "time_pid=$TIME_PID"
    echo "solver_pid=$SOLVER_PID"
    echo "process_group=$PROCESS_GROUP"
    echo "starttime=$STARTTIME"
    echo "cmdline_sha256=$CMDLINE_SHA256"
    echo "watchdog_pid=$WATCHDOG_PID"
    echo "deadline_epoch=$DEADLINE_EPOCH"
    echo "binary_sha256=$BINARY_SHA256"
    echo "watchdog_sha256=$WATCHDOG_SHA256"
    echo "plan_sha256=$PLAN_SHA256"
    echo "preflight_sha256=$PREFLIGHT_SHA256"
    echo "launch_sha256=$(sha256sum "$0" | awk '{print $1}')"
} >"$LOG_DIR/launch-identity.txt"

waited=0
while ! grep -q '^attempt=1 clock=64 ' "$LOG_DIR/stdout"; do
    [ "$waited" -lt 1200 ] || {
        echo "first-step record timeout" >>"$LOG_DIR/first-step-gate.txt"
        stop_solver
        exit 82
    }
    solver_identity_matches || { echo "solver exited or identity changed before first step" >&2; exit 83; }
    sleep 5
    waited=$((waited + 5))
done
first=$(grep '^attempt=1 clock=64 ' "$LOG_DIR/stdout" | head -n 1)
printf '%s\n' "$first" >"$LOG_DIR/first-step.txt"
printf '%s\n' "$first" | grep -q 'rhs_timed_calls=12 ' || { stop_solver; exit 84; }
printf '%s\n' "$first" | grep -q 'cache_hit_miss=\[7, 5\] ' || { stop_solver; exit 85; }
printf '%s\n' "$first" | grep -q 'steady_allocations=0$' || { stop_solver; exit 86; }
first_attempt=$OUTPUT/step-001-clock-0064/attempt.json
[ -r "$first_attempt" ] || { stop_solver; exit 89; }
expected_identity=$(sed -n 's/.* profile_identity=\(.*\) archive_profile=.*/\1/p' "$LOG_DIR/preflight.stdout")
[ -n "$expected_identity" ] || { stop_solver; exit 90; }
grep -Fq "\"identity\": \"$expected_identity\"" "$first_attempt" || { stop_solver; exit 91; }
first_seconds=$(printf '%s\n' "$first" | sed -n 's/.*integration_seconds=\([0-9.]*\).*/\1/p')
awk -v actual="$first_seconds" -v limit="$FIRST_STEP_LIMIT_SECONDS" 'BEGIN{exit !(actual <= limit)}' || {
    echo "first-step integration exceeded reviewed limit: $first_seconds" >>"$LOG_DIR/first-step-gate.txt"
    stop_solver
    exit 87
}
now=$(date +%s)
[ "$((DEADLINE_EPOCH - now))" -ge "$REMAINING_AFTER_FIRST_SECONDS" ] || {
    echo "insufficient post-first-step deadline margin" >>"$LOG_DIR/first-step-gate.txt"
    stop_solver
    exit 88
}
printf 'passed utc=%s integration_seconds=%s remaining_seconds=%s\n' "$(date -u +%FT%TZ)" "$first_seconds" "$((DEADLINE_EPOCH - now))" >"$LOG_DIR/first-step-gate.txt"
echo "first-step gate passed; solver remains under the identity-bound watchdog"
