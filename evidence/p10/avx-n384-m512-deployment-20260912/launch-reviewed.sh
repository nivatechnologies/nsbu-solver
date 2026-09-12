#!/bin/sh
set -eu

SOURCE=326eeb5cbd5ebe39a7d5f7be77f9acfab8d0db72
BINARY_SHA256=85659a06a1b914d7b64feec20522eb47e876a17c626c3703d4ef59b6cb379b57
WATCHDOG_SHA256=4b65e13b74bd7a32237044b5467d4f223c8dc3e6e35d6e8d3498e1938fa53e4b
CLEANUP_SHA256=d1317f52922bcd20525d26bb5cf817c3cc7fe8715c5d372fe4b09a1913a549f7
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
PREDECESSOR_SOURCE=aed49b7d7874a0a720dee88b65ba180c7286fa65
PREDECESSOR_PROFILE=n384-m384-h64to2048-h128to4096-cadv33-w3-f13c29c

[ "${NSBU_LAUNCH_N384_M512:-}" = 1 ] || {
    echo "root must set NSBU_LAUNCH_N384_M512=1 for this exact reviewed launch" >&2
    exit 64
}
[ "$(hostname)" = sulaco ] || { echo "host must be sulaco" >&2; exit 65; }

BUNDLE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
BIN=$BUNDLE/p10-avx-n384-m512-piecewise-cadv33
WATCHDOG=$BUNDLE/pgid-watchdog-v2.sh
CLEANUP=$BUNDLE/launcher-owned-cleanup.sh
PLAN=$BUNDLE/frozen-plan.json
RUN_ROOT=$BUNDLE/run
OUTPUT=$RUN_ROOT/output
LOG_DIR=$RUN_ROOT/logs

[ ! -e "$RUN_ROOT" ] || { echo "fresh run directory required" >&2; exit 66; }
[ "$(sha256sum "$BIN" | awk '{print $1}')" = "$BINARY_SHA256" ] || exit 67
[ "$(sha256sum "$WATCHDOG" | awk '{print $1}')" = "$WATCHDOG_SHA256" ] || exit 68
[ "$(sha256sum "$CLEANUP" | awk '{print $1}')" = "$CLEANUP_SHA256" ] || exit 94
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
python3 - "$PREDECESSOR" "$PREDECESSOR_SOURCE" "$PREDECESSOR_PROFILE" <<'PY'
import glob
import json
import math
import os
import sys

root, source, profile = sys.argv[1:]
expected = {
    512: 8,
    1024: 16,
    1536: 24,
    2048: 32,
    2560: 36,
    3072: 40,
    3584: 44,
    4096: 48,
}
for clock, attempt_number in expected.items():
    paths = glob.glob(os.path.join(root, "output", f"step-{attempt_number:03d}-clock-{clock:04d}"))
    if len(paths) != 1:
        raise SystemExit(f"observer screen clock {clock}: expected one step directory")
    step = paths[0]
    if os.path.getsize(os.path.join(step, "state.bin")) != 1_366_033_529:
        raise SystemExit(f"observer screen clock {clock}: unexpected state size")
    with open(os.path.join(step, "attempt.json"), encoding="utf-8") as stream:
        attempt = json.load(stream)
    with open(os.path.join(step, "record.json"), encoding="utf-8") as stream:
        record = json.load(stream)
    identity = attempt.get("identity", "")
    if f"source={source};" not in identity or f"profile={profile};" not in identity:
        raise SystemExit(f"observer screen clock {clock}: predecessor identity mismatch")
    if (attempt.get("attempt"), attempt.get("attempted_to"), attempt.get("outcome")) != (attempt_number, clock, "committed"):
        raise SystemExit(f"observer screen clock {clock}: attempt mismatch")
    if (attempt.get("rhs_calls"), attempt.get("cache_hits"), attempt.get("cache_misses"), attempt.get("steady_allocations")) != (12, 7, 5, 0):
        raise SystemExit(f"observer screen clock {clock}: execution counters mismatch")
    if record.get("observation_status") != "Scheduled" or record.get("clock") != clock:
        raise SystemExit(f"observer screen clock {clock}: observation record mismatch")
    values = [
        attempt.get("observer_seconds"),
        attempt.get("error_ratio_l2"),
        attempt.get("error_ratio_h1"),
        *record.get("timing", {}).values(),
        *record.get("balance", {}).values(),
    ]
    if not values or any(not isinstance(value, (int, float)) or not math.isfinite(value) or value < 0 for value in values):
        raise SystemExit(f"observer screen clock {clock}: non-finite or negative measurement")
    if attempt["error_ratio_l2"] > 1 or attempt["error_ratio_h1"] > 1:
        raise SystemExit(f"observer screen clock {clock}: rejected error ratio")
print("predecessor_observer_screen=passed nodes=8")
PY
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

LAUNCH_STARTED=0
LAUNCH_HANDOFF=0
TIME_PID=
TIME_STARTTIME=
TIME_CMDLINE_SHA256=
SOLVER_PID=
PROCESS_GROUP=
STARTTIME=
CMDLINE_SHA256=
. "$CLEANUP"

setsid /usr/bin/time -v -o "$LOG_DIR/time.txt" "$BIN" run "$OUTPUT" >"$LOG_DIR/stdout" 2>"$LOG_DIR/stderr" &
TIME_PID=$!
time_stat=$(cat "/proc/$TIME_PID/stat")
time_rest=${time_stat##*) }
set -- $time_rest
PROCESS_GROUP=$3
TIME_STARTTIME=${20}
TIME_CMDLINE_SHA256=$(sha256sum "/proc/$TIME_PID/cmdline" | awk '{print $1}')
LAUNCH_STARTED=1
install_owned_cleanup_traps
[ "$PROCESS_GROUP" = "$TIME_PID" ] || exit 95
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
watchdog_started="started leader_pid=$SOLVER_PID process_group=$PROCESS_GROUP starttime=$STARTTIME cmdline_sha256=$CMDLINE_SHA256 deadline_epoch=$DEADLINE_EPOCH"
WATCHDOG_CONFIRMED=0
watchdog_alive() {
    [ -r "/proc/$WATCHDOG_PID/stat" ] || return 1
    watchdog_stat=$(cat "/proc/$WATCHDOG_PID/stat" 2>/dev/null) || return 1
    watchdog_rest=${watchdog_stat##*) }
    set -- $watchdog_rest
    [ "$1" != Z ]
}
for _ in $(seq 1 100); do
    if grep -Fq "$watchdog_started" "$LOG_DIR/watchdog.log" 2>/dev/null && watchdog_alive; then
        WATCHDOG_CONFIRMED=1
        break
    fi
    watchdog_alive || break
    sleep 0.05
done
[ "$WATCHDOG_CONFIRMED" -eq 1 ] || exit 96

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
        cleanup_owned_group
        exit 82
    }
    owned_member_matches "$SOLVER_PID" "$PROCESS_GROUP" "$STARTTIME" "$CMDLINE_SHA256" || {
        echo "solver exited or identity changed before first step" >&2
        exit 83
    }
    watchdog_alive || { echo "watchdog exited before first step" >&2; exit 97; }
    sleep 5
    waited=$((waited + 5))
done
watchdog_alive || { cleanup_owned_group; exit 97; }
first=$(grep '^attempt=1 clock=64 ' "$LOG_DIR/stdout" | head -n 1)
printf '%s\n' "$first" >"$LOG_DIR/first-step.txt"
printf '%s\n' "$first" | grep -q 'rhs_timed_calls=12 ' || { cleanup_owned_group; exit 84; }
printf '%s\n' "$first" | grep -q 'cache_hit_miss=\[7, 5\] ' || { cleanup_owned_group; exit 85; }
printf '%s\n' "$first" | grep -q 'steady_allocations=0$' || { cleanup_owned_group; exit 86; }
first_attempt=$OUTPUT/step-001-clock-0064/attempt.json
[ -r "$first_attempt" ] || { cleanup_owned_group; exit 89; }
expected_identity=$(sed -n 's/.* profile_identity=\(.*\) archive_profile=.*/\1/p' "$LOG_DIR/preflight.stdout")
[ -n "$expected_identity" ] || { cleanup_owned_group; exit 90; }
grep -Fq "\"identity\": \"$expected_identity\"" "$first_attempt" || { cleanup_owned_group; exit 91; }
first_seconds=$(printf '%s\n' "$first" | sed -n 's/.*integration_seconds=\([0-9.]*\).*/\1/p')
awk -v actual="$first_seconds" -v limit="$FIRST_STEP_LIMIT_SECONDS" 'BEGIN{exit !(actual <= limit)}' || {
    echo "first-step integration exceeded reviewed limit: $first_seconds" >>"$LOG_DIR/first-step-gate.txt"
    cleanup_owned_group
    exit 87
}
now=$(date +%s)
[ "$((DEADLINE_EPOCH - now))" -ge "$REMAINING_AFTER_FIRST_SECONDS" ] || {
    echo "insufficient post-first-step deadline margin" >>"$LOG_DIR/first-step-gate.txt"
    cleanup_owned_group
    exit 88
}
printf 'passed utc=%s integration_seconds=%s remaining_seconds=%s\n' "$(date -u +%FT%TZ)" "$first_seconds" "$((DEADLINE_EPOCH - now))" >"$LOG_DIR/first-step-gate.txt"
LAUNCH_HANDOFF=1
echo "first-step gate passed; solver remains under the identity-bound watchdog"
