#!/bin/bash
set -euo pipefail

readonly SOURCE_COMMIT=2a2db1e07faa9c806a00dd4ff36db87de41e5b6f
readonly BINARY_SHA256=ff5456f81007dc7a605cebaa2150c13d8a22c2e8ecf574f5122d8e4b58966ed6
readonly PLAN_SHA256=4bf2e35d24dd1ad231139fd07992bd691691f8cfe22dc446167163994dba4b63
readonly CAP_BYTES=137438953472
readonly MIN_AVAILABLE_KIB=150994944
readonly PROJECTION_PID=1509730
readonly INPUT_ROOT=/mnt/niva-array/p10-offline-residual-probe-m512-input-20260913

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo=$(CDPATH= cd -- "$script_dir/../../.." && pwd)
binary="$script_dir/harness/target/release/p10-offline-residual-probe"
plan="$script_dir/m512-probe-plan.json"
run_dir="$script_dir/raw/m512-localization-r5-20260913"

if [[ ${1-} == --worker ]]; then
    shift
    [[ $# == 1 && $1 == "$run_dir" ]] || exit 64
    ulimit -v 134217728
    printf '%s\n' "$CAP_BYTES" > "$run_dir/run.rlimit-as-bytes"
    date -u +'%Y-%m-%dT%H:%M:%SZ' > "$run_dir/run.start-utc"
    set +e
    /usr/bin/time -v -o "$run_dir/run.time" \
        timeout --foreground --signal=TERM --kill-after=60s 2700s \
        "$binary" run "$plan" "$INPUT_ROOT" "$CAP_BYTES" \
        > "$run_dir/run.stdout" 2> "$run_dir/run.stderr"
    status=$?
    set -e
    printf '%s\n' "$status" > "$run_dir/run.status"
    date -u +'%Y-%m-%dT%H:%M:%SZ' > "$run_dir/run.end-utc"
    exit "$status"
fi

[[ ${PROJECTION_RELEASE_CONFIRMED-} == 1 ]] || {
    echo 'refusing launch without PROJECTION_RELEASE_CONFIRMED=1' >&2
    exit 64
}
[[ ! -e /proc/$PROJECTION_PID ]] || {
    echo "refusing launch while projection PID $PROJECTION_PID exists" >&2
    exit 64
}
available_kib=$(awk '$1 == "MemAvailable:" { print $2 }' /proc/meminfo)
[[ $available_kib =~ ^[0-9]+$ && $available_kib -ge $MIN_AVAILABLE_KIB ]] || {
    echo "refusing launch: MemAvailable ${available_kib:-unknown} KiB below $MIN_AVAILABLE_KIB KiB" >&2
    exit 64
}
[[ $(sha256sum "$binary" | awk '{ print $1 }') == "$BINARY_SHA256" ]]
[[ $(sha256sum "$plan" | awk '{ print $1 }') == "$PLAN_SHA256" ]]
git -C "$repo" merge-base --is-ancestor "$SOURCE_COMMIT" HEAD
git -C "$repo" diff --quiet "$SOURCE_COMMIT" -- \
    evidence/p10/offline-residual-probe/harness \
    evidence/p10/offline-residual-probe/m512-probe-plan.json
git -C "$repo" diff --quiet -- \
    evidence/p10/offline-residual-probe/harness \
    evidence/p10/offline-residual-probe/m512-probe-plan.json

sha256sum --check --status <<EOF
5ce6a433cb45c19561eec3ca653b6062cba40ef5537b3687e8081f43c78b797e  $INPUT_ROOT/step-017-clock-1088/state.bin
6eab4cd0980c612bedb83790412c80fb9cab06dede669e03aacc644330467df8  $INPUT_ROOT/step-018-clock-1152/state.bin
9927e92775d09bd5c620c2852913e31c36c64bcd1add47c401302d029e32c2ca  $INPUT_ROOT/step-019-clock-1216/state.bin
EOF

mkdir -p "$script_dir/raw"
mkdir "$run_dir"
setsid "$script_dir/launch-m512-localization.sh" --worker "$run_dir" </dev/null &
owner_pid=$!
expected_cmdline_sha256=$(
    printf '/bin/bash\0%s\0--worker\0%s\0' \
        "$script_dir/launch-m512-localization.sh" "$run_dir" | sha256sum | awk '{ print $1 }'
)
same_bound_owner() {
    [[ -n ${bound_starttime-} && -r /proc/$owner_pid/stat && -r /proc/$owner_pid/cmdline ]] || return 1
    [[ $(awk '{ print $22 }' "/proc/$owner_pid/stat") == "$bound_starttime" ]] || return 1
    [[ $(ps -o pgid= -p "$owner_pid" | tr -d ' ') == "$owner_pid" ]] || return 1
    [[ $(sha256sum "/proc/$owner_pid/cmdline" | awk '{ print $1 }') == "$expected_cmdline_sha256" ]]
}
stable=0
for _ in {1..200}; do
    if [[ -r /proc/$owner_pid/stat && -r /proc/$owner_pid/cmdline ]]; then
        owner_pgid=$(ps -o pgid= -p "$owner_pid" | tr -d ' ')
        owner_cmdline_sha256=$(sha256sum "/proc/$owner_pid/cmdline" | awk '{ print $1 }')
        if [[ $owner_pgid == "$owner_pid" && $owner_cmdline_sha256 == "$expected_cmdline_sha256" ]]; then
            bound_starttime=$(awk '{ print $22 }' "/proc/$owner_pid/stat")
            stable=$((stable + 1))
            [[ $stable == 2 ]] && break
        else
            stable=0
        fi
    fi
    sleep 0.01
done
if [[ $stable != 2 ]]; then
    if same_bound_owner; then
        kill -TERM -- "-$owner_pid" 2>/dev/null || true
        deadline=$((SECONDS + 60))
        while same_bound_owner && ((SECONDS < deadline)); do
            sleep 1
        done
        if same_bound_owner; then
            kill -KILL -- "-$owner_pid" 2>/dev/null || true
            echo 'worker identity did not stabilize; bound identity received TERM then KILL' > "$run_dir/launch-refusal.txt"
        else
            echo 'worker identity did not stabilize; bound identity received TERM and exited or changed' > "$run_dir/launch-refusal.txt"
        fi
    else
        echo 'worker identity did not stabilize; no identity bound and no signal sent; active timeout retained' > "$run_dir/launch-refusal.txt"
    fi
    exit 64
fi
owner_starttime=$bound_starttime
cat > "$run_dir/launch-receipt.json" <<EOF
{
  "schema": "p10-offline-residual-m512-launch-receipt-v1",
  "source_commit": "$SOURCE_COMMIT",
  "binary_sha256": "$BINARY_SHA256",
  "plan_sha256": "$PLAN_SHA256",
  "cap_bytes": $CAP_BYTES,
  "mem_available_kib": $available_kib,
  "minimum_mem_available_bytes": 154618822656,
  "timeout_seconds": 2700,
  "kill_grace_seconds": 60,
  "projection_pid_confirmed_absent": $PROJECTION_PID,
  "owner_pid": $owner_pid,
  "owner_pgid": $owner_pgid,
  "owner_starttime": $owner_starttime,
  "owner_cmdline_sha256": "$owner_cmdline_sha256",
  "expected_owner_cmdline_sha256": "$expected_cmdline_sha256",
  "run_dir": "$run_dir"
}
EOF
printf 'launched run_dir=%s owner_pid=%s owner_pgid=%s owner_starttime=%s\n' \
    "$run_dir" "$owner_pid" "$owner_pgid" "$owner_starttime"
