#!/bin/sh
set -eu

BUNDLE=/tmp/nsbu-p10-sulaco-m512-r6-force-clock2560-20260913
BIN_SHA256=9d3d473109e32dac98582aae7bd286ffc38016cebd46c82adb1e12346d1ff2be
LEFT_MANIFEST_SHA256=038e02b7d97cd6bda168407d040bc8398a65156c9b3c9290aebd726516c0cb49
RIGHT_MANIFEST_SHA256=acbed0801b7001d467205a5bf15077a1cbd241d6ca97919d686efd465ae86da7
LEFT_FILE_SHA256=12e4b8818008461cdd95b4d7532d1e4acd3fac4ba41aaeacc6cda8f74ef70963
RIGHT_FILE_SHA256=c2d73550c5ee1465800fc43780e74ea5307b8bdcf7f395e8cbcad7b0c14e15c6
LEFT_COEFF_SHA256=a45c2c950bd800956058f921795a325de0f37c40fd8e35f05f1e43e0984a76b4
RIGHT_COEFF_SHA256=bb675ddbf0217d397df4f137250c8ff870454e07965b41b642d7a3a19f766406
LEFT_PLAN_SHA256=2c20dbfede51b2ad9ce3f64e2d2ded818eb38a19534e2379da8204560047fb9a
RIGHT_PLAN_SHA256=2be3880204aab5da1819e11ed6abb377e43b814f8ef17869d76463f72a33cf84
INTERNAL_CAP_BYTES=2733113344
EXTERNAL_VMEM_CAP_KIB=8388608
MEMORY_FLOOR_KIB=16777216
DISK_FLOOR_BYTES=1073741824
DEADLINE_EPOCH=1789329174
TIMEOUT_SECONDS=600
KILL_GRACE_SECONDS=30
REQUIRED_REMAINING_SECONDS=660

[ "${NSBU_LAUNCH_M512_R6_FORCE_CLOCK2560:-}" = 1 ] || exit 64
[ "$(hostname)" = sulaco ] || exit 65
SELF_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
[ "$SELF_DIR" = "$BUNDLE" ] || exit 66
BIN=$BUNDLE/p10-snapshot-comparison-adapter
LEFT=$BUNDLE/force-clock2560-left.json
RIGHT=$BUNDLE/force-clock2560-right.json
RUN=$BUNDLE/run
[ ! -e "$RUN" ] || exit 67
[ "$(sha256sum "$BIN" | awk '{print $1}')" = "$BIN_SHA256" ] || exit 68
[ "$(sha256sum "$LEFT" | awk '{print $1}')" = "$LEFT_MANIFEST_SHA256" ] || exit 69
[ "$(sha256sum "$RIGHT" | awk '{print $1}')" = "$RIGHT_MANIFEST_SHA256" ] || exit 70

validate_side() {
    manifest=$1 file_sha=$2 coeff_sha=$3 plan_sha=$4
    snapshot=$(jq -er .snapshot "$manifest")
    record=${snapshot%/state.bin}/record.json
    plan=$(jq -er .plan "$manifest")
    [ "$(jq -er .comparison_kind "$manifest")" = FORCE_RESOLUTION_DIAGNOSTIC ] || exit 71
    [ "$(jq -er .evolution.comparison_endpoint "$manifest")" = 2560 ] || exit 72
    [ "$(jq -er .elapsed "$manifest")" = 2560 ] || exit 73
    [ "$(sha256sum "$snapshot" | awk '{print $1}')" = "$file_sha" ] || exit 74
    [ "$(sha256sum "$plan" | awk '{print $1}')" = "$plan_sha" ] || exit 75
    [ "$(jq -er .state_sha256 "$record")" = "$coeff_sha" ] || exit 76
    [ "$(jq -er .coefficient_sha256 "$manifest")" = "$coeff_sha" ] || exit 77
    [ "$(jq -er .identity "$record")" = "$(jq -er .identity "$manifest")" ] || exit 78
    [ "$(jq -er .clock "$record")" = 2560 ] || exit 79
}
validate_side "$LEFT" "$LEFT_FILE_SHA256" "$LEFT_COEFF_SHA256" "$LEFT_PLAN_SHA256"
validate_side "$RIGHT" "$RIGHT_FILE_SHA256" "$RIGHT_COEFF_SHA256" "$RIGHT_PLAN_SHA256"
[ "$(awk '/MemAvailable/{print $2}' /proc/meminfo)" -ge "$MEMORY_FLOOR_KIB" ] || exit 80
[ "$(df -B1 --output=avail "$BUNDLE" | awk 'NR==2{print $1}')" -ge "$DISK_FLOOR_BYTES" ] || exit 81
[ "$((DEADLINE_EPOCH - $(date +%s)))" -ge "$REQUIRED_REMAINING_SECONDS" ] || exit 82

mkdir "$RUN"
ulimit -v "$EXTERNAL_VMEM_CAP_KIB"
partial=$RUN/clock2560.partial.json
pid= pgid= starttime= expected_cmdline_sha256=
owned_running() {
    [ -n "$pid" ] && [ -n "$pgid" ] && [ -n "$starttime" ] && [ -n "$expected_cmdline_sha256" ] || return 1
    [ -r "/proc/$pid/stat" ] && [ -r "/proc/$pid/cmdline" ] || return 1
    [ "$pgid" = "$pid" ] || return 1
    [ "$(awk '{print $5}' "/proc/$pid/stat")" = "$pgid" ] || return 1
    [ "$(awk '{print $22}' "/proc/$pid/stat")" = "$starttime" ] || return 1
    [ "$(sha256sum "/proc/$pid/cmdline" | awk '{print $1}')" = "$expected_cmdline_sha256" ]
}
cleanup() {
    status=$?
    trap - EXIT HUP INT TERM
    if [ "$status" -ne 0 ] && owned_running; then
        /bin/kill -TERM -- "-$pgid" 2>/dev/null || true
        for _ in $(seq 1 "$KILL_GRACE_SECONDS"); do owned_running || break; sleep 1; done
        owned_running && /bin/kill -KILL -- "-$pgid" 2>/dev/null || true
    fi
    exit "$status"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM
expected_cmdline_sha256=$(
    printf '%s\0' /usr/bin/time -v -o "$RUN/clock2560.time" \
        /usr/bin/timeout --foreground --signal=TERM --kill-after="${KILL_GRACE_SECONDS}s" \
        "$TIMEOUT_SECONDS" "$BIN" "$LEFT" "$RIGHT" "$INTERNAL_CAP_BYTES" |
        sha256sum | awk '{print $1}'
)
/usr/bin/setsid /usr/bin/time -v -o "$RUN/clock2560.time" \
    /usr/bin/timeout --foreground --signal=TERM --kill-after="${KILL_GRACE_SECONDS}s" \
    "$TIMEOUT_SECONDS" "$BIN" "$LEFT" "$RIGHT" "$INTERNAL_CAP_BYTES" \
    >"$partial" 2>"$RUN/clock2560.stderr" &
pid=$!
stable=0
pgid= starttime=
for _ in $(seq 1 100); do
    if [ -r "/proc/$pid/stat" ] && [ -r "/proc/$pid/cmdline" ]; then
        pgid=$(awk '{print $5}' "/proc/$pid/stat")
        starttime=$(awk '{print $22}' "/proc/$pid/stat")
        observed=$(sha256sum "/proc/$pid/cmdline" | awk '{print $1}')
        if [ "$pgid" = "$pid" ] && [ "$observed" = "$expected_cmdline_sha256" ]; then
            stable=$((stable + 1))
            [ "$stable" -ge 2 ] && break
        else
            stable=0
        fi
    fi
    sleep 0.05
done
[ "$stable" -ge 2 ] || exit 83
printf 'leader_pid=%s\nprocess_group=%s\nstarttime=%s\ncmdline_sha256=%s\ntimeout_seconds=%s\nkill_grace_seconds=%s\ndeadline_epoch=%s\n' \
    "$pid" "$pgid" "$starttime" "$expected_cmdline_sha256" "$TIMEOUT_SECONDS" "$KILL_GRACE_SECONDS" "$DEADLINE_EPOCH" >"$RUN/clock2560.launch-identity"
set +e
wait "$pid"
status=$?
set -e
printf '%s\n' "$status" >"$RUN/clock2560.status"
[ "$status" -eq 0 ] || exit "$status"
pid= pgid= starttime= expected_cmdline_sha256=
mv "$partial" "$RUN/clock2560.json"
trap - EXIT HUP INT TERM
