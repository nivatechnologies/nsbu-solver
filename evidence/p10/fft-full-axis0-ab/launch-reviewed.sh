#!/bin/bash
set -euo pipefail
readonly LIBRARY_SOURCE=4b5a6709b3ce521428a3502b666fc9ec0cb6cae0
readonly HARNESS_SOURCE=d9972fe973a97382451b92165ea64f0a0e182f63
readonly BINARY_SHA256=dad7653776b8207a24bbe387b53e135896ecbee0c98c1d1eb4542dc62efdfa08
readonly OPENSSL_SHA256=30cc7c491903d6d8bca54406889c0334a167777397458214b5bd498c51b6fd97
readonly AXIS12_BYTES=51925833024
readonly FULL_BYTES=62825780832
readonly AS_BYTES=68719476736
readonly MIN_AVAILABLE_KIB=94907734
readonly MIN_DISK_BYTES=16777216
readonly MAX_ARTIFACT_BYTES=4194304
readonly TOTAL_WALL_SECONDS=1500
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo=$(CDPATH= cd -- "$script_dir/../../.." && pwd)
binary="$script_dir/harness/target/release/p10-fft-full-axis0-ab"
run_dir="$script_dir/raw/ab-openssl-20260914"

if [[ ${1-} == --worker ]]; then
    shift
    [[ $# == 3 ]] || exit 64
    mode=$1; round=$2; round_dir=$3
    [[ $mode == axis12 || $mode == full ]]
    [[ $round_dir == "$run_dir/$round" && -d $round_dir ]]
    ulimit -v 67108864
    printf '%s\n' "$AS_BYTES" > "$round_dir/run.rlimit-as-bytes"
    date -u +'%Y-%m-%dT%H:%M:%SZ' > "$round_dir/run.start-utc"
    set +e
    /usr/bin/time -v -o "$round_dir/run.time" \
        timeout --foreground --signal=TERM --kill-after=30s 300s \
        "$binary" "$mode" > "$round_dir/run.stdout" 2> "$round_dir/run.stderr"
    status=$?
    set -e
    printf '%s\n' "$status" > "$round_dir/run.status"
    date -u +'%Y-%m-%dT%H:%M:%SZ' > "$round_dir/run.end-utc"
    exit "$status"
fi

check_bindings() {
    git -C "$repo" merge-base --is-ancestor "$HARNESS_SOURCE" HEAD
    git -C "$repo" diff --quiet "$LIBRARY_SOURCE" -- crates/nsbu-solver
    git -C "$repo" diff --quiet "$HARNESS_SOURCE" -- evidence/p10/fft-full-axis0-ab/harness
    [[ $(sha256sum "$binary" | awk '{print $1}') == "$BINARY_SHA256" ]]
    [[ $(sha256sum /usr/bin/openssl | awk '{print $1}') == "$OPENSSL_SHA256" ]]
    [[ $("$binary" axis12 --preflight) == *\"resource_bytes\":$AXIS12_BYTES* ]]
    [[ $("$binary" full --preflight) == *\"resource_bytes\":$FULL_BYTES* ]]
}
check_host() {
    available_kib=$(awk '$1=="MemAvailable:" {print $2}' /proc/meminfo)
    [[ $available_kib =~ ^[0-9]+$ && $available_kib -ge $MIN_AVAILABLE_KIB ]]
    disk_bytes=$(df -PB1 "$script_dir" | awk 'NR==2 {print $4}')
    [[ $disk_bytes =~ ^[0-9]+$ && $disk_bytes -ge $MIN_DISK_BYTES ]]
    if pgrep -f '[p]10-fft-full-axis0-ab (axis12|full)' >/dev/null; then
        echo 'refusing concurrent FFT A/B process' >&2; exit 64
    fi
    (( $(date -u +%s) + 330 <= end_epoch ))
}
same_owner() {
    [[ -n ${bound_starttime-} && -r /proc/$owner_pid/stat && -r /proc/$owner_pid/cmdline ]] || return 1
    [[ $(awk '{print $22}' /proc/$owner_pid/stat) == "$bound_starttime" ]]
    [[ $(ps -o pgid= -p "$owner_pid" | tr -d ' ') == "$owner_pid" ]]
    [[ $(sha256sum /proc/$owner_pid/cmdline | awk '{print $1}') == "$expected_cmdline" ]]
}
launch_round() {
    mode=$1; round=$2; round_dir="$run_dir/$round"
    check_bindings; check_host; mkdir "$round_dir"
    setsid "$script_dir/launch-reviewed.sh" --worker "$mode" "$round" "$round_dir" </dev/null &
    owner_pid=$!; bound_starttime=; stable=0
    expected_cmdline=$(printf '/bin/bash\0%s\0--worker\0%s\0%s\0%s\0' "$script_dir/launch-reviewed.sh" "$mode" "$round" "$round_dir" | sha256sum | awk '{print $1}')
    for _ in {1..200}; do
        if [[ -r /proc/$owner_pid/stat && -r /proc/$owner_pid/cmdline ]] &&
           [[ $(ps -o pgid= -p "$owner_pid" | tr -d ' ') == "$owner_pid" ]] &&
           [[ $(sha256sum /proc/$owner_pid/cmdline | awk '{print $1}') == "$expected_cmdline" ]]; then
            bound_starttime=$(awk '{print $22}' /proc/$owner_pid/stat); stable=$((stable+1)); [[ $stable == 2 ]] && break
        else stable=0; fi
        sleep 0.01
    done
    if [[ $stable != 2 ]]; then
        if same_owner; then kill -TERM -- "-$owner_pid" 2>/dev/null || true; fi
        echo 'identity did not stabilize; only a bound owner was eligible for signaling' > "$round_dir/launch-refusal.txt"
        exit 64
    fi
    cat > "$round_dir/identity.json" <<EOF
{"mode":"$mode","round":"$round","owner_pid":$owner_pid,"owner_pgid":$owner_pid,"owner_starttime":$bound_starttime,"cmdline_sha256":"$expected_cmdline"}
EOF
    set +e; wait "$owner_pid"; status=$?; set -e
    [[ $status == 0 ]]
    python3 - "$round_dir/run.stdout" <<'PY'
import json,sys
x=json.load(open(sys.argv[1]))
assert x['allocations']==[0,0,0]
assert len(set(x['forward_sha256']))==1 and len(set(x['inverse_sha256']))==1
PY
    [[ $(du -sb "$run_dir" | awk '{print $1}') -le $MAX_ARTIFACT_BYTES ]]
}

[[ ${FULL_AXIS0_AB_AUTHORIZED-} == 1 ]] || { echo 'authorization variable absent' >&2; exit 64; }
[[ $# == 0 || ${1-} == --preflight-only ]] || exit 64
start_epoch=$(date -u +%s); end_epoch=$((start_epoch + TOTAL_WALL_SECONDS))
check_bindings; check_host
if [[ ${1-} == --preflight-only ]]; then
    echo "preflight passed axis12=$AXIS12_BYTES full=$FULL_BYTES available_kib=$available_kib"; exit 0
fi
mkdir -p "$script_dir/raw"; mkdir "$run_dir"
cat > "$run_dir/launch-receipt.json" <<EOF
{"schema":"p10-fft-full-axis0-ab-launch-v1","library_source":"$LIBRARY_SOURCE","harness_source":"$HARNESS_SOURCE","binary_sha256":"$BINARY_SHA256","openssl_sha256":"$OPENSSL_SHA256","axis12_resource_bytes":$AXIS12_BYTES,"full_resource_bytes":$FULL_BYTES,"address_space_bytes":$AS_BYTES,"minimum_available_kib":$MIN_AVAILABLE_KIB,"prelaunch_available_kib":$available_kib,"prelaunch_disk_bytes":$disk_bytes,"total_wall_seconds":$TOTAL_WALL_SECONDS,"order":["axis12-a","full-a","axis12-b","full-b"]}
EOF
launch_round axis12 axis12-a
launch_round full full-a
python3 - "$run_dir/axis12-a/run.stdout" "$run_dir/full-a/run.stdout" <<'PY'
import json,statistics,sys
a,b=(json.load(open(p)) for p in sys.argv[1:])
assert b['forward_sha256']==a['forward_sha256'] and b['inverse_sha256']==a['inverse_sha256']
assert statistics.median(b['forward_ns']) <= 2*statistics.median(a['forward_ns'])
assert statistics.median(b['inverse_ns']) <= 2*statistics.median(a['inverse_ns'])
PY
launch_round axis12 axis12-b
launch_round full full-b
python3 - "$run_dir" <<'PY'
import json,sys
p=sys.argv[1]; names=['axis12-a','full-a','axis12-b','full-b']
x=[json.load(open(f'{p}/{n}/run.stdout')) for n in names]
assert all(v['allocations']==[0,0,0] for v in x)
assert all(v['forward_sha256']==x[0]['forward_sha256'] and v['inverse_sha256']==x[0]['inverse_sha256'] for v in x)
PY
date -u +'%Y-%m-%dT%H:%M:%SZ' > "$run_dir/run.end-utc"
