#!/bin/sh
# Proposed only: a future deadline and explicit authorization are required.
set -eu

SOURCE=e25f3816f83c6a7c07202cac2878f58ace460511
BINARY_SHA256=2c9ae9401733a5062eb65e096803b7eea3075fa11d6ad87388d8c30b7ca49143
PLAN_SHA256=338197a8d033c6eb435554f8a5ce4533f1a6a18cc606b72bd367781e067c8651
PREFLIGHT_SHA256=b8180201ae96fca1856e032f36756eaab8292d8a1ad993b7f838218ee54887e6
WATCHDOG_SHA256=e07cc2b1a4e6375523f08c50dc176289a36d560f1d0af4b3a61eae02873c50f1
ARCHIVE_HELPER_SHA256=df8d9ad38454b0a42edb90b6adb287c02af2b0ffa2060c67680b7934eef49386
MEMORY_FLOOR_BYTES=241937824240
DISK_FLOOR_BYTES=189586276352
ADDRESS_SPACE_LIMIT_KIB=268435456
MINIMUM_LAUNCH_REMAINING_SECONDS=66672
REMAINING_AFTER_FIRST_SECONDS=65410
FIRST_STEP_INTEGRATION_LIMIT_SECONDS=1200
STATE_COEFFICIENT_BYTES=3233808384
PREPARED_BINARY_EXTERNAL_STOP=pgid-watchdog-v3-confirmed-identity-absolute-deadline
REQUIRED_EXTERNAL_STOP=pgid-watchdog-v3-confirmed-identity-absolute-deadline

numeric() { case "$1" in '' | *[!0-9]*) return 1 ;; esac; }

validate_deadline() {
    numeric "$1" || { echo "refused: numeric future absolute deadline required" >&2; return 72; }
    deadline=$1
    now=$2
    [ "$deadline" -gt "$now" ] || { echo "refused: absolute deadline has passed" >&2; return 72; }
    [ "$((deadline - now))" -ge "$MINIMUM_LAUNCH_REMAINING_SECONDS" ] || {
        echo "refused: deadline leaves less than reviewed run budget" >&2
        return 73
    }
}

require_v3_identity() {
    [ "$PREPARED_BINARY_EXTERNAL_STOP" = "$REQUIRED_EXTERNAL_STOP" ] || {
        echo "refused: rebuild and refreeze binary identity for the reviewed v3 watchdog" >&2
        return 63
    }
}

owned_pid=
owned_pgid=
owned_starttime=
owned_cmdline=
owned_matches() {
    [ -n "$owned_pid" ] && [ -r "/proc/$owned_pid/stat" ] && [ -r "/proc/$owned_pid/cmdline" ] || return 1
    owned_stat=$(cat "/proc/$owned_pid/stat" 2>/dev/null) || return 1
    owned_rest=${owned_stat##*) }
    set -- $owned_rest
    [ "$1" != Z ] && [ "$3" = "$owned_pgid" ] && [ "${20}" = "$owned_starttime" ] || return 1
    [ "$(sha256sum "/proc/$owned_pid/cmdline" | awk '{print $1}')" = "$owned_cmdline" ]
}
cleanup_owned() {
    status=$?
    trap - EXIT HUP INT TERM
    if [ "$status" -ne 0 ] && owned_matches; then
        stop_owned "${CLEANUP_GRACE_SECONDS:-60}"
    fi
    exit "$status"
}

stop_owned() {
    grace=$1
    /bin/kill -TERM -- "-$owned_pgid" 2>/dev/null || true
    seconds=0
    while [ "$seconds" -lt "$grace" ] && owned_matches; do sleep 1; seconds=$((seconds + 1)); done
    if owned_matches; then /bin/kill -KILL -- "-$owned_pgid" 2>/dev/null || true; fi
    wait "$owned_pid" 2>/dev/null || true
}

acquire_owned_identity() {
    candidate=$1
    expected_program=$2
    count=0
    while [ "$count" -lt 200 ]; do
        if [ -r "/proc/$candidate/stat" ] && [ -r "/proc/$candidate/cmdline" ]; then
            first=$(tr '\000' '\n' <"/proc/$candidate/cmdline" 2>/dev/null | head -n 1)
            stat=$(cat "/proc/$candidate/stat" 2>/dev/null || true)
            rest=${stat##*) }; set -- $rest
            if [ "$#" -ge 20 ] && [ "$1" != Z ] && [ "$3" = "$candidate" ] && [ "$first" = "$expected_program" ]; then
                candidate_start=${20}
                candidate_cmd=$(sha256sum "/proc/$candidate/cmdline" | awk '{print $1}')
                sleep 0.05
                stat2=$(cat "/proc/$candidate/stat" 2>/dev/null || true)
                rest2=${stat2##*) }; set -- $rest2
                if [ "$#" -ge 20 ] && [ "$1" != Z ] && [ "$3" = "$candidate" ] && [ "${20}" = "$candidate_start" ] && [ "$(sha256sum "/proc/$candidate/cmdline" | awk '{print $1}')" = "$candidate_cmd" ]; then
                    owned_pid=$candidate; owned_pgid=$candidate; owned_starttime=$candidate_start; owned_cmdline=$candidate_cmd
                    return 0
                fi
            fi
        fi
        sleep 0.05; count=$((count + 1))
    done
    /bin/kill -TERM "$candidate" 2>/dev/null || true
    sleep 1
    /bin/kill -KILL "$candidate" 2>/dev/null || true
    wait "$candidate" 2>/dev/null || true
    return 1
}

acquire_group_member_identity() {
    candidate=$1
    expected_program=$2
    expected_pgid=$3
    count=0
    while [ "$count" -lt 200 ]; do
        if [ -r "/proc/$candidate/stat" ] && [ -r "/proc/$candidate/cmdline" ]; then
            first=$(tr '\000' '\n' <"/proc/$candidate/cmdline" 2>/dev/null | head -n 1)
            stat=$(cat "/proc/$candidate/stat" 2>/dev/null || true)
            rest=${stat##*) }; set -- $rest
            if [ "$#" -ge 20 ] && [ "$1" != Z ] && [ "$3" = "$expected_pgid" ] && [ "$first" = "$expected_program" ]; then
                candidate_start=${20}
                candidate_cmd=$(sha256sum "/proc/$candidate/cmdline" | awk '{print $1}')
                sleep 0.05
                stat2=$(cat "/proc/$candidate/stat" 2>/dev/null || true)
                rest2=${stat2##*) }; set -- $rest2
                if [ "$#" -ge 20 ] && [ "$1" != Z ] && [ "$3" = "$expected_pgid" ] && [ "${20}" = "$candidate_start" ] && [ "$(sha256sum "/proc/$candidate/cmdline" | awk '{print $1}')" = "$candidate_cmd" ]; then
                    owned_pid=$candidate; owned_pgid=$expected_pgid; owned_starttime=$candidate_start; owned_cmdline=$candidate_cmd
                    return 0
                fi
            fi
        fi
        sleep 0.05; count=$((count + 1))
    done
    return 1
}

fake_owner_test() {
    temp=$(mktemp -d "${TMPDIR:-/tmp}/nsbu-n512-watchdog.XXXXXX")
    setsid /bin/sleep 30 &
    child=$!
    pgid=
    for _ in $(seq 1 100); do
        stat=$(cat "/proc/$child/stat" 2>/dev/null || true)
        rest=${stat##*) }
        set -- $rest
        if [ "$#" -ge 20 ] && [ "$3" = "$child" ]; then
            pgid=$3
            starttime=${20}
            cmdline=$(sha256sum "/proc/$child/cmdline" | awk '{print $1}')
            break
        fi
        sleep 0.05
    done
    [ -n "$pgid" ] || return 1
    WATCHDOG_POLL_SECONDS=1 "$WATCHDOG" "$child" "$pgid" "$starttime" "$cmdline" "$(( $(date +%s) + 1 ))" "$temp/watchdog.log"
    wait "$child" 2>/dev/null || true
    [ ! -r "/proc/$child/stat" ] || { echo "fake owner survived watchdog" >&2; return 1; }
    grep -q 'deadline_reached_sending_TERM' "$temp/watchdog.log"
    echo "fake-child owner/deadline test passed"
}

fake_handshake_test() {
    setsid /bin/sh -c 'sleep 30' & child=$!
    acquire_owned_identity "$child" /bin/sh
    [ "$owned_pgid" = "$child" ]
    stop_owned 1
    [ ! -r "/proc/$child/stat" ]
    owned_pid=
    echo "post-setsid stable identity handshake passed"
}

fake_archive_timeout_test() {
    temp=$(mktemp -d "${TMPDIR:-/tmp}/nsbu-n512-archive-timeout.XXXXXX")
    setsid /bin/sh -c 'trap "" TERM; while :; do sleep 1; done' & child=$!
    acquire_owned_identity "$child" /bin/sh
    WATCHDOG_POLL_SECONDS=1 WATCHDOG_GRACE_SECONDS=1 "$WATCHDOG" "$owned_pid" "$owned_pgid" "$owned_starttime" "$owned_cmdline" "$(( $(date +%s) + 1 ))" "$temp/watchdog.log"
    wait "$child" 2>/dev/null || true
    [ ! -r "/proc/$child/stat" ]
    grep -q 'grace_expired_sending_KILL' "$temp/watchdog.log"
    owned_pid=
    echo "TERM-ignoring archive deadline escalation passed"
}

fake_wrapper_cleanup_test() {
    temp=$(mktemp -d "${TMPDIR:-/tmp}/nsbu-n512-wrapper-cleanup.XXXXXX")
    setsid /bin/sh -c 'trap "exit 0" TERM; /bin/sh -c '\''trap "" TERM; while :; do sleep 1; done'\'' & echo $! >"$1"; wait' wrapper "$temp/child" &
    wrapper=$!
    count=0
    while [ ! -s "$temp/child" ] && [ "$count" -lt 100 ]; do sleep 0.05; count=$((count + 1)); done
    child=$(cat "$temp/child")
    acquire_group_member_identity "$child" /bin/sh "$wrapper"
    stop_owned 1
    wait "$wrapper" 2>/dev/null || true
    count=0
    while [ -r "/proc/$child/stat" ] && [ "$count" -lt 100 ]; do sleep 0.05; count=$((count + 1)); done
    [ ! -r "/proc/$child/stat" ]
    owned_pid=
    echo "wrapper exit plus TERM-ignoring solver cleanup passed"
}

fake_archive_collision_test() {
    temp=$(mktemp -d "${TMPDIR:-/tmp}/nsbu-n512-archive-collision.XXXXXX")
    mkdir "$temp/output" "$temp/archive" "$temp/logs"
    printf x >"$temp/output/state"
    if "$ARCHIVE_HELPER" "$temp/output" "$temp/partial" "$temp/archive" "$temp/logs"; then return 1; fi
    [ -d "$temp/partial" ] && [ -d "$temp/archive" ]
    echo "preexisting archive target refused with partial preserved"
}

BUNDLE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
WATCHDOG=$BUNDLE/pgid-watchdog-v3.sh
ARCHIVE_HELPER=$BUNDLE/archive-local.sh
case "${1:-}" in
    --self-test-deadline-refusal)
        test_now=$(date +%s)
        if validate_deadline invalid "$test_now"; then exit 1; fi
        if validate_deadline 1 "$test_now"; then exit 1; fi
        if validate_deadline "$((test_now + MINIMUM_LAUNCH_REMAINING_SECONDS - 1))" "$test_now"; then exit 1; fi
        validate_deadline "$((test_now + MINIMUM_LAUNCH_REMAINING_SECONDS))" "$test_now"
        echo "numeric, expired, short, and admitted deadline tests passed"
        exit 0
        ;;
    --self-test-owner) fake_owner_test; exit $? ;;
    --self-test-handshake) fake_handshake_test; exit $? ;;
    --self-test-archive-timeout) fake_archive_timeout_test; exit $? ;;
    --self-test-wrapper-cleanup) fake_wrapper_cleanup_test; exit $? ;;
    --self-test-archive-collision) fake_archive_collision_test; exit $? ;;
    --self-test-identity) require_v3_identity; echo "v3 binary identity admitted"; exit 0 ;;
esac

[ "${NSBU_LAUNCH_N512_M512_ENDPOINT:-}" = 1 ] || {
    echo "refused: caller must explicitly authorize this future launch" >&2
    exit 64
}
[ "$(hostname)" = sulaco ] || { echo "refused: Sulaco only" >&2; exit 65; }
[ -n "${NSBU_N512_M512_EXPECTED_BUNDLE:-}" ] && [ "$BUNDLE" = "$NSBU_N512_M512_EXPECTED_BUNDLE" ] || {
    echo "refused: caller-supplied exact bundle path mismatch" >&2
    exit 66
}
DEADLINE_EPOCH=${NSBU_N512_M512_DEADLINE_EPOCH:-}
validate_deadline "$DEADLINE_EPOCH" "$(date +%s)"
require_v3_identity
ARCHIVE_PARENT=${NSBU_N512_M512_ARCHIVE_PARENT:-}
case "$ARCHIVE_PARENT" in /*) ;; *) echo "refused: absolute archive parent required" >&2; exit 67 ;; esac
[ -d "$ARCHIVE_PARENT" ] || { echo "refused: archive parent missing" >&2; exit 67; }

BIN=$BUNDLE/p10-avx-scheduled-endpoint
PLAN=$BUNDLE/v3-launch-plan.json
PREFLIGHT=$BUNDLE/v3-n512-preflight.stdout
RUN_ROOT=$BUNDLE/run-$DEADLINE_EPOCH
OUTPUT=$RUN_ROOT/output
LOG_DIR=$RUN_ROOT/logs
ARCHIVE=$ARCHIVE_PARENT/n512-m512-endpoint-$DEADLINE_EPOCH
ARCHIVE_PARTIAL=$ARCHIVE.partial

[ ! -e "$RUN_ROOT" ] && [ ! -e "$ARCHIVE" ] && [ ! -e "$ARCHIVE_PARTIAL" ] || {
    echo "refused: run or archive target already exists" >&2
    exit 68
}
[ "$(sha256sum "$BIN" | awk '{print $1}')" = "$BINARY_SHA256" ] || exit 69
[ "$(sha256sum "$PLAN" | awk '{print $1}')" = "$PLAN_SHA256" ] || exit 69
[ "$(sha256sum "$PREFLIGHT" | awk '{print $1}')" = "$PREFLIGHT_SHA256" ] || exit 69
[ "$(sha256sum "$WATCHDOG" | awk '{print $1}')" = "$WATCHDOG_SHA256" ] || exit 69
[ "$(sha256sum "$ARCHIVE_HELPER" | awk '{print $1}')" = "$ARCHIVE_HELPER_SHA256" ] || exit 69
if pgrep -f 'p10-avx-scheduled-endpoint run ' >/dev/null; then
    echo "refused: competing endpoint solver exists" >&2
    exit 70
fi
available_kib=$(awk '/MemAvailable/{print $2}' /proc/meminfo)
[ "$((available_kib * 1024))" -ge "$MEMORY_FLOOR_BYTES" ] || exit 74
source_disk=$(df -B1 --output=avail "$BUNDLE" | awk 'NR==2{print $1}')
archive_disk=$(df -B1 --output=avail "$ARCHIVE_PARENT" | awk 'NR==2{print $1}')
[ "$source_disk" -ge "$DISK_FLOOR_BYTES" ] && [ "$archive_disk" -ge "$DISK_FLOOR_BYTES" ] || exit 75

mkdir "$RUN_ROOT" || { echo "refused: run root won launch race" >&2; exit 68; }
mkdir "$LOG_DIR"
"$BIN" preflight unused >"$LOG_DIR/preflight.stdout"
[ "$(sha256sum "$LOG_DIR/preflight.stdout" | awk '{print $1}')" = "$PREFLIGHT_SHA256" ] || exit 76

timeout_seconds=$((DEADLINE_EPOCH - $(date +%s)))
ulimit -v "$ADDRESS_SPACE_LIMIT_KIB"
export NSBU_RUN_N512_M512_ENDPOINT_CAPTURE=1
setsid /usr/bin/time -v -o "$LOG_DIR/time.txt" /usr/bin/timeout --foreground --signal=TERM --kill-after=60s "$timeout_seconds" "$BIN" run "$OUTPUT" >"$LOG_DIR/stdout" 2>"$LOG_DIR/stderr" &
time_pid=$!
acquire_owned_identity "$time_pid" /usr/bin/time || { echo "refused: solver owner identity handshake failed" >&2; exit 77; }
time_pgid=$owned_pgid
trap cleanup_owned EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

solver_pid=
for _ in $(seq 1 100); do
    timeout_pid=$(pgrep -P "$time_pid" || true)
    [ -n "$timeout_pid" ] && solver_pid=$(pgrep -P "$timeout_pid" || true) && [ -n "$solver_pid" ] && break
    sleep 0.05
done
[ -n "$solver_pid" ] || exit 77
stat=$(cat "/proc/$solver_pid/stat")
rest=${stat##*) }
set -- $rest
pgid=$3
starttime=${20}
cmdline=$(sha256sum "/proc/$solver_pid/cmdline" | awk '{print $1}')
[ "$pgid" = "$time_pgid" ] || exit 77
acquire_group_member_identity "$solver_pid" "$BIN" "$time_pgid" || { echo "refused: solver child identity handshake failed" >&2; exit 77; }
pgid=$owned_pgid
starttime=$owned_starttime
cmdline=$owned_cmdline
setsid "$WATCHDOG" "$solver_pid" "$pgid" "$starttime" "$cmdline" "$DEADLINE_EPOCH" "$LOG_DIR/watchdog.log" >"$LOG_DIR/watchdog.stdout" 2>"$LOG_DIR/watchdog.stderr" &
watchdog_pid=$!

waited=0
while ! grep -q '^attempt=1 clock=64 ' "$LOG_DIR/stdout" 2>/dev/null; do
    [ "$waited" -lt 1500 ] && kill -0 "$solver_pid" 2>/dev/null && kill -0 "$watchdog_pid" 2>/dev/null || {
        /bin/kill -TERM -- "-$pgid" 2>/dev/null || true
        exit 78
    }
    sleep 5
    waited=$((waited + 5))
done
first=$(grep '^attempt=1 clock=64 ' "$LOG_DIR/stdout" | head -n 1)
printf '%s\n' "$first" | grep -q 'rhs_timed_calls=12 ' || exit 79
printf '%s\n' "$first" | grep -q 'cache_hit_miss=\[7, 5\] ' || exit 79
printf '%s\n' "$first" | grep -q 'steady_allocations=0$' || exit 79
first_seconds=$(printf '%s\n' "$first" | sed -n 's/.*integration_seconds=\([0-9.]*\).*/\1/p')
awk -v actual="$first_seconds" -v limit="$FIRST_STEP_INTEGRATION_LIMIT_SECONDS" 'BEGIN{exit !(actual ~ /^[0-9]+([.][0-9]+)?$/ && actual <= limit)}' || exit 80
[ "$((DEADLINE_EPOCH - $(date +%s)))" -ge "$REMAINING_AFTER_FIRST_SECONDS" ] || exit 81

if wait "$time_pid"; then solver_status=0; else solver_status=$?; fi
[ "$solver_status" -eq 0 ] || exit "$solver_status"
wait "$watchdog_pid" 2>/dev/null || true
owned_pid=
grep -q '^terminal endpoint_capture_complete_offline_observer_required clock=4096$' "$LOG_DIR/stdout" || exit 82
python3 - "$OUTPUT" "$STATE_COEFFICIENT_BYTES" <<'PY'
import json, pathlib, sys
root, coefficient_bytes = pathlib.Path(sys.argv[1]), int(sys.argv[2])
clocks = [64 * i for i in range(1, 33)] + [2048 + 128 * i for i in range(1, 17)]
observer = {512, 1024, 1536, 2048, 2560, 3072, 3584, 4096}
for attempt, clock in enumerate(clocks, 1):
    step = root / f"step-{attempt:03d}-clock-{clock:04d}"
    if not step.is_dir(): raise SystemExit(f"missing {step.name}")
    with (step / "attempt.json").open() as stream: attempted = json.load(stream)
    with (step / "record.json").open() as stream: record = json.load(stream)
    expected = (attempt, clock, "committed", 12, 7, 5, 0)
    actual = (attempted.get("attempt"), attempted.get("attempted_to"), attempted.get("outcome"), attempted.get("rhs_calls"), attempted.get("cache_hits"), attempted.get("cache_misses"), attempted.get("steady_allocations"))
    if actual != expected: raise SystemExit(f"attempt identity mismatch {attempt}: {actual}")
    if record.get("clock") != clock or record.get("coefficient_bytes") != coefficient_bytes: raise SystemExit(f"state record mismatch {attempt}")
    if record.get("offline_observer_node") != (clock in observer) or record.get("qualification") is not False: raise SystemExit(f"observer marker mismatch {attempt}")
    if (step / "state.bin").stat().st_size <= coefficient_bytes: raise SystemExit(f"state payload truncated {attempt}")
print("all_48_committed_states=passed offline_observer_nodes=8")
PY

archive_disk=$(df -B1 --output=avail "$ARCHIVE_PARENT" | awk 'NR==2{print $1}')
[ "$archive_disk" -ge "$DISK_FLOOR_BYTES" ] || exit 75
setsid "$ARCHIVE_HELPER" "$OUTPUT" "$ARCHIVE_PARTIAL" "$ARCHIVE" "$LOG_DIR" >"$LOG_DIR/archive.stdout" 2>"$LOG_DIR/archive.stderr" &
archive_pid=$!
acquire_owned_identity "$archive_pid" /bin/sh || { echo "refused: archive owner identity handshake failed" >&2; exit 83; }
setsid "$WATCHDOG" "$owned_pid" "$owned_pgid" "$owned_starttime" "$owned_cmdline" "$DEADLINE_EPOCH" "$LOG_DIR/archive-watchdog.log" >"$LOG_DIR/archive-watchdog.stdout" 2>"$LOG_DIR/archive-watchdog.stderr" &
archive_watchdog_pid=$!
if wait "$archive_pid"; then archive_status=0; else archive_status=$?; fi
[ "$archive_status" -eq 0 ] || exit "$archive_status"
owned_pid=
wait "$archive_watchdog_pid" 2>/dev/null || true
echo "endpoint and archive complete; offline baccus observer remains required"
