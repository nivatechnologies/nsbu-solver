#!/bin/sh
set -eu

BUNDLE=/tmp/nsbu-p10-sulaco-m512-r6-force-comparisons-20260913
BIN_SHA256=9d3d473109e32dac98582aae7bd286ffc38016cebd46c82adb1e12346d1ff2be
LEFT3584_MANIFEST_SHA256=51ef4dd2346622382fb1d014f6a55360ce4f036a5e17a134650f4bd75db0bd48
RIGHT3584_MANIFEST_SHA256=71029ede82ee9dfea3d2ba0af240b22f4f1d3a39fad0355c2620f142a5541aa2
LEFT4096_MANIFEST_SHA256=8485f526274711ac6cf0c18d0ecc2135e22f4085f1b31c17cc4b7af8feeff2d7
RIGHT4096_MANIFEST_SHA256=72f148b50c5d2c877d838a1b5d576edc7f2d44bdad925d2bbfb06436138ca5f4
LEFT3584_FILE_SHA256=a8ff93feb0950af68a9cc93e5a62e59fe8f3538e94555be70bda6e7c8403a004
RIGHT3584_FILE_SHA256=7389d17c82c56acd0878408dca73be59675b62e8f8d25b372781dd1e771f187d
LEFT4096_FILE_SHA256=2868bc6e5ccbfb5ce5967aefd3d82cccbaec0a72baaeaad0944f517948187592
RIGHT4096_FILE_SHA256=43308ca8ddb499916a09183266e3a83811b3214536c14c7ba2f6814363cef0a0
LEFT3584_COEFF_SHA256=a1490c09bff7c7fa191a845785e94420882eafb1842bbb95dff9f5a42f4f97bc
RIGHT3584_COEFF_SHA256=267c1b92de5e4562ae96dc6fcc05813a9c49c6c5238e03add5154503d4afa721
LEFT4096_COEFF_SHA256=921e2e3b83eea4b259d9794eb0e663ce8f8f913322283312997820e31a1cb72b
RIGHT4096_COEFF_SHA256=1d1500409962c4af37182f6733c2c88a086247c8e8728570e9c93238ed1c04fd
LEFT_PLAN_SHA256=2c20dbfede51b2ad9ce3f64e2d2ded818eb38a19534e2379da8204560047fb9a
RIGHT_PLAN_SHA256=2be3880204aab5da1819e11ed6abb377e43b814f8ef17869d76463f72a33cf84
INTERNAL_CAP_BYTES=2733113344
EXTERNAL_VMEM_CAP_KIB=8388608
MEMORY_FLOOR_KIB=16777216
DISK_FLOOR_BYTES=1073741824
DEADLINE_EPOCH=1789329174
COMPARISON_TIMEOUT_SECONDS=600
KILL_GRACE_SECONDS=30
CLEANUP_RESERVE_SECONDS=30
REQUIRED_REMAINING_SECONDS=660

[ "${NSBU_LAUNCH_M512_R6_FORCE_COMPARISONS:-}" = 1 ] || {
    echo "refused: exact root authorization variable required" >&2
    exit 64
}
[ "$(hostname)" = sulaco ] || { echo "refused: Sulaco only" >&2; exit 65; }
SELF_DIR=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
[ "$SELF_DIR" = "$BUNDLE" ] || { echo "refused: comparison stage mismatch" >&2; exit 66; }
case "$LEFT4096_MANIFEST_SHA256:$RIGHT4096_MANIFEST_SHA256:$RIGHT4096_FILE_SHA256:$RIGHT4096_COEFF_SHA256" in
    *__PENDING_*) echo "refused: clock4096 bindings not frozen" >&2; exit 67 ;;
esac

BIN=$BUNDLE/p10-snapshot-comparison-adapter
LEFT3584=$BUNDLE/force-clock3584-left.json
RIGHT3584=$BUNDLE/force-clock3584-right.json
LEFT4096=$BUNDLE/force-clock4096-left.json
RIGHT4096=$BUNDLE/force-clock4096-right.json
RUN=$BUNDLE/run

[ ! -e "$RUN" ] || { echo "refused: fresh comparison output required" >&2; exit 68; }
[ "$(sha256sum "$BIN" | awk '{print $1}')" = "$BIN_SHA256" ] || exit 69
[ "$(sha256sum "$LEFT3584" | awk '{print $1}')" = "$LEFT3584_MANIFEST_SHA256" ] || exit 70
[ "$(sha256sum "$RIGHT3584" | awk '{print $1}')" = "$RIGHT3584_MANIFEST_SHA256" ] || exit 71
[ "$(sha256sum "$LEFT4096" | awk '{print $1}')" = "$LEFT4096_MANIFEST_SHA256" ] || exit 72
[ "$(sha256sum "$RIGHT4096" | awk '{print $1}')" = "$RIGHT4096_MANIFEST_SHA256" ] || exit 73

owned_running() {
    pid=$1
    expected_starttime=$2
    [ -r "/proc/$pid/stat" ] || return 1
    [ "$(awk '{print $22}' "/proc/$pid/stat")" = "$expected_starttime" ]
}
owned_running 260583 37283148 && { echo "refused: owned r6 solver still live" >&2; exit 74; }
owned_running 260597 37283171 && { echo "refused: owned r6 watchdog still live" >&2; exit 75; }

admit_comparison() {
    available_kib=$(awk '/MemAvailable/{print $2}' /proc/meminfo)
    [ "$available_kib" -ge "$MEMORY_FLOOR_KIB" ] || { echo "refused: memory floor" >&2; return 76; }
    available_disk=$(df -B1 --output=avail "$BUNDLE" | awk 'NR==2{print $1}')
    [ "$available_disk" -ge "$DISK_FLOOR_BYTES" ] || { echo "refused: disk floor" >&2; return 77; }
    remaining=$((DEADLINE_EPOCH - $(date +%s)))
    [ "$remaining" -ge "$REQUIRED_REMAINING_SECONDS" ] || {
        echo "refused: insufficient time for 600s comparison, 30s kill grace and 30s cleanup reserve" >&2
        return 78
    }
}

validate_side() {
    manifest=$1
    expected_clock=$2
    expected_file_sha=$3
    expected_coeff_sha=$4
    expected_plan_sha=$5
    snapshot=$(jq -er '.snapshot' "$manifest")
    record=${snapshot%/state.bin}/record.json
    plan=$(jq -er '.plan' "$manifest")
    [ "$(jq -er '.comparison_kind' "$manifest")" = FORCE_RESOLUTION_DIAGNOSTIC ] || exit 79
    [ "$(jq -er '.evolution.comparison_endpoint' "$manifest")" = "$expected_clock" ] || exit 80
    [ "$(jq -er '.elapsed' "$manifest")" = "$expected_clock" ] || exit 81
    [ "$(sha256sum "$snapshot" | awk '{print $1}')" = "$expected_file_sha" ] || exit 82
    [ "$(sha256sum "$plan" | awk '{print $1}')" = "$expected_plan_sha" ] || exit 83
    [ "$(jq -er '.state_sha256' "$record")" = "$expected_coeff_sha" ] || exit 84
    [ "$(jq -er '.coefficient_sha256' "$manifest")" = "$expected_coeff_sha" ] || exit 85
    [ "$(jq -er '.identity' "$record")" = "$(jq -er '.identity' "$manifest")" ] || exit 86
    [ "$(jq -er '.clock' "$record")" = "$expected_clock" ] || exit 87
}

validate_side "$LEFT3584" 3584 "$LEFT3584_FILE_SHA256" "$LEFT3584_COEFF_SHA256" "$LEFT_PLAN_SHA256"
validate_side "$RIGHT3584" 3584 "$RIGHT3584_FILE_SHA256" "$RIGHT3584_COEFF_SHA256" "$RIGHT_PLAN_SHA256"
validate_side "$LEFT4096" 4096 "$LEFT4096_FILE_SHA256" "$LEFT4096_COEFF_SHA256" "$LEFT_PLAN_SHA256"
validate_side "$RIGHT4096" 4096 "$RIGHT4096_FILE_SHA256" "$RIGHT4096_COEFF_SHA256" "$RIGHT_PLAN_SHA256"

mkdir "$RUN"
ulimit -v "$EXTERNAL_VMEM_CAP_KIB"

pending_pid=
pending_pgid=
pending_starttime=
pending_cmdline_sha256=

pending_matches() {
    [ -n "$pending_pid" ] && [ -r "/proc/$pending_pid/stat" ] && [ -r "/proc/$pending_pid/cmdline" ] || return 1
    [ "$(awk '{print $5}' "/proc/$pending_pid/stat")" = "$pending_pgid" ] || return 1
    [ "$(awk '{print $22}' "/proc/$pending_pid/stat")" = "$pending_starttime" ] || return 1
    [ "$(sha256sum "/proc/$pending_pid/cmdline" | awk '{print $1}')" = "$pending_cmdline_sha256" ]
}

cleanup_pending() {
    pending_matches || return 0
    /bin/kill -TERM -- "-$pending_pgid" 2>/dev/null || true
    for _ in $(seq 1 "$KILL_GRACE_SECONDS"); do pending_matches || return 0; sleep 1; done
    pending_matches && /bin/kill -KILL -- "-$pending_pgid" 2>/dev/null || true
}

cleanup() {
    status=$?
    trap - EXIT HUP INT TERM
    [ "$status" -eq 0 ] || cleanup_pending
    exit "$status"
}
trap cleanup EXIT
trap 'exit 129' HUP
trap 'exit 130' INT
trap 'exit 143' TERM

run_comparison() {
    label=$1
    left=$2
    right=$3
    admit_comparison
    partial=$RUN/$label.partial.json
    expected_cmdline_sha256=$(
        printf '%s\0' /usr/bin/time -v -o "$RUN/$label.time" \
            /usr/bin/timeout --foreground --signal=TERM --kill-after="${KILL_GRACE_SECONDS}s" \
            "$COMPARISON_TIMEOUT_SECONDS" "$BIN" "$left" "$right" "$INTERNAL_CAP_BYTES" |
            sha256sum | awk '{print $1}'
    )
    /usr/bin/setsid /usr/bin/time -v -o "$RUN/$label.time" \
        /usr/bin/timeout --foreground --signal=TERM --kill-after="${KILL_GRACE_SECONDS}s" \
        "$COMPARISON_TIMEOUT_SECONDS" "$BIN" "$left" "$right" "$INTERNAL_CAP_BYTES" \
        >"$partial" 2>"$RUN/$label.stderr" &
    pending_pid=$!
    stable_samples=0
    for _ in $(seq 1 100); do
        [ -r "/proc/$pending_pid/stat" ] && [ -r "/proc/$pending_pid/cmdline" ] || { sleep 0.05; continue; }
        pending_pgid=$(awk '{print $5}' "/proc/$pending_pid/stat")
        pending_starttime=$(awk '{print $22}' "/proc/$pending_pid/stat")
        pending_cmdline_sha256=$expected_cmdline_sha256
        if [ "$pending_pgid" = "$pending_pid" ] && pending_matches; then
            stable_samples=$((stable_samples + 1))
            [ "$stable_samples" -ge 2 ] && break
            sleep 0.05
            continue
        fi
        stable_samples=0
        pending_pgid=
        pending_starttime=
        pending_cmdline_sha256=
        sleep 0.05
    done
    pending_matches || { echo "failed to bind comparison leader identity" >&2; return 88; }
    printf 'label=%s\nleader_pid=%s\nprocess_group=%s\nstarttime=%s\ncmdline_sha256=%s\ntimeout_seconds=%s\nkill_grace_seconds=%s\ndeadline_epoch=%s\n' \
        "$label" "$pending_pid" "$pending_pgid" "$pending_starttime" "$pending_cmdline_sha256" \
        "$COMPARISON_TIMEOUT_SECONDS" "$KILL_GRACE_SECONDS" "$DEADLINE_EPOCH" >"$RUN/$label.launch-identity"
    set +e
    wait "$pending_pid"
    status=$?
    set -e
    printf '%s\n' "$status" >"$RUN/$label.status"
    if [ "$status" -ne 0 ]; then
        cleanup_pending
        return "$status"
    fi
    pending_pid=
    pending_pgid=
    pending_starttime=
    pending_cmdline_sha256=
    mv "$partial" "$RUN/$label.json"
}

run_comparison clock3584 "$LEFT3584" "$RIGHT3584"
run_comparison clock4096 "$LEFT4096" "$RIGHT4096"

trap - EXIT HUP INT TERM
printf 'completed clocks=3584,4096 deadline_epoch=%s internal_cap_bytes=%s external_vmem_cap_kib=%s comparison_timeout_seconds=%s kill_grace_seconds=%s cleanup_reserve_seconds=%s\n' \
    "$DEADLINE_EPOCH" "$INTERNAL_CAP_BYTES" "$EXTERNAL_VMEM_CAP_KIB" \
    "$COMPARISON_TIMEOUT_SECONDS" "$KILL_GRACE_SECONDS" "$CLEANUP_RESERVE_SECONDS"
