#!/bin/bash
set -euo pipefail

readonly TILED_SOURCE=335f6741c21e5b56409ca47e335b1bbb920a60e9
readonly UNTILED_SOURCE=ca4256930c86f59864631a888d990cb9ec9d6ab8
readonly TILED_BINARY_SHA256=5ad30a91ecf3ae8c1dad34cfe52d06cbeaa9694c618e40d06019fab2d711c797
readonly UNTILED_BINARY_SHA256=759bdb12a76c410c183f73cbcd2410ab6c1d12797938fcc1b94f6652cb4bc3b1
readonly FORCE_AS_KIB=67108864
readonly RHS_AS_KIB=100663296
readonly FORCE_MIN_AVAILABLE_KIB=83886080
readonly RHS_MIN_AVAILABLE_KIB=117440512
readonly MAX_ARTIFACT_BYTES=4194304
readonly MIN_DISK_AVAILABLE_BYTES=8388608
readonly CAMPAIGN_DEADLINE_UTC=2026-09-13T19:52:54Z
readonly CAMPAIGN_DEADLINE_EPOCH=1789329174

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
tiled_repo=$(CDPATH= cd -- "$script_dir/../../.." && pwd)
untiled_repo=/mnt/niva-array/nsbu-solver/work/p10-fft-untiled-w3-controls-20260913
tiled_binary="$script_dir/harness/target/release/p10-w3-768-controls"
untiled_binary="$untiled_repo/evidence/p10/fft-transverse-tile-w3-controls/harness/target/release/p10-w3-768-controls"
run_dir="$script_dir/raw/controls-20260913"

if [[ ${1-} == --worker ]]; then
    shift
    [[ $# == 4 ]] || exit 64
    variant=$1
    control=$2
    round=$3
    round_dir=$4
    [[ $round_dir == "$run_dir/$round" && -d $round_dir ]] || exit 64
    case $variant in
        tiled) binary=$tiled_binary ;;
        untiled) binary=$untiled_binary ;;
        *) exit 64 ;;
    esac
    case $control in
        force) as_kib=$FORCE_AS_KIB; timeout_seconds=900; gate=NSBU_RUN_W3_768_FORCE_CONTROL ;;
        rhs) as_kib=$RHS_AS_KIB; timeout_seconds=1800; gate=NSBU_RUN_W3_768_RHS_CONTROL ;;
        *) exit 64 ;;
    esac
    ulimit -v "$as_kib"
    printf '%s\n' "$((as_kib * 1024))" > "$round_dir/run.rlimit-as-bytes"
    date -u +'%Y-%m-%dT%H:%M:%SZ' > "$round_dir/run.start-utc"
    set +e
    /usr/bin/time -v -o "$round_dir/run.time" \
        timeout --foreground --signal=TERM --kill-after=60s "${timeout_seconds}s" \
        env "$gate=1" "$binary" "$control" \
        > "$round_dir/run.stdout" 2> "$round_dir/run.stderr"
    status=$?
    set -e
    printf '%s\n' "$status" > "$round_dir/run.status"
    date -u +'%Y-%m-%dT%H:%M:%SZ' > "$round_dir/run.end-utc"
    exit "$status"
fi

check_quiet_host() {
    if pgrep -f '[p]rojection-n512|[p]10-n512-m768-one-attempt|[p]10-w3-768-controls (force|rhs)' >/dev/null; then
        echo 'refusing launch while a conflicting numerical command remains' >&2
        exit 64
    fi
}

check_bindings() {
    [[ $(git -C "$tiled_repo" rev-parse --verify "$TILED_SOURCE") == "$TILED_SOURCE" ]]
    git -C "$tiled_repo" diff --quiet "$TILED_SOURCE" -- crates/nsbu-solver crates/nsbu-benchmarks
    git -C "$tiled_repo" diff --quiet -- crates/nsbu-solver crates/nsbu-benchmarks
    [[ $(git -C "$untiled_repo" rev-parse HEAD) == "$UNTILED_SOURCE" ]]
    git -C "$untiled_repo" diff --quiet -- crates/nsbu-solver crates/nsbu-benchmarks
    for source in Cargo.toml Cargo.lock src/main.rs src/tests.rs; do
        cmp -s "$script_dir/harness/$source" \
            "$untiled_repo/evidence/p10/fft-transverse-tile-w3-controls/harness/$source"
    done
    [[ $(sha256sum "$tiled_binary" | awk '{ print $1 }') == "$TILED_BINARY_SHA256" ]]
    [[ $(sha256sum "$untiled_binary" | awk '{ print $1 }') == "$UNTILED_BINARY_SHA256" ]]
    "$tiled_binary" preflight | grep -q '^status=admitted_preallocation variant=tiled '
    "$untiled_binary" preflight | grep -q '^status=admitted_preallocation variant=untiled '
}

check_available() {
    local minimum=$1
    available_kib=$(awk '$1 == "MemAvailable:" { print $2 }' /proc/meminfo)
    [[ $available_kib =~ ^[0-9]+$ && $available_kib -ge $minimum ]] || {
        echo "refusing launch: MemAvailable ${available_kib:-unknown} KiB below $minimum KiB" >&2
        exit 64
    }
}

check_deadline() {
    local remaining_seconds=$1
    local now_epoch
    now_epoch=$(date -u +%s)
    ((now_epoch + remaining_seconds + 300 <= CAMPAIGN_DEADLINE_EPOCH)) || {
        echo "refusing launch: remaining controls plus preparation exceed $CAMPAIGN_DEADLINE_UTC" >&2
        exit 64
    }
}

check_disk() {
    disk_available_bytes=$(df -PB1 "$script_dir" | awk 'NR == 2 { print $4 }')
    [[ $disk_available_bytes =~ ^[0-9]+$ && $disk_available_bytes -ge $MIN_DISK_AVAILABLE_BYTES ]] || {
        echo "refusing launch: disk available ${disk_available_bytes:-unknown} below $MIN_DISK_AVAILABLE_BYTES bytes" >&2
        exit 64
    }
}

check_quiet_host
check_bindings
check_available "$RHS_MIN_AVAILABLE_KIB"
check_deadline 5640
check_disk
if [[ ${1-} == --preflight-only ]]; then
    echo "preflight passed MemAvailable=${available_kib}KiB deadline=$CAMPAIGN_DEADLINE_UTC"
    exit 0
fi
[[ $# == 0 ]] || exit 64
[[ ${CONTROL_LAUNCH_AUTHORIZED-} == 1 ]] || {
    echo 'refusing launch without CONTROL_LAUNCH_AUTHORIZED=1' >&2
    exit 64
}
mkdir -p "$script_dir/raw"
mkdir "$run_dir"
cat > "$run_dir/launch-receipt.json" <<EOF
{"schema":"p10-fft-tile-w3-cross-source-launch-v1","tiled_source":"$TILED_SOURCE","untiled_source":"$UNTILED_SOURCE","tiled_binary_sha256":"$TILED_BINARY_SHA256","untiled_binary_sha256":"$UNTILED_BINARY_SHA256","force_as_bytes":$((FORCE_AS_KIB * 1024)),"rhs_as_bytes":$((RHS_AS_KIB * 1024)),"prelaunch_mem_available_kib":$available_kib,"prelaunch_disk_available_bytes":$disk_available_bytes,"campaign_deadline_utc":"$CAMPAIGN_DEADLINE_UTC","order":["untiled-force","tiled-force","untiled-rhs","tiled-rhs"]}
EOF

same_bound_owner() {
    [[ -n ${bound_starttime-} && -r /proc/$owner_pid/stat && -r /proc/$owner_pid/cmdline ]] || return 1
    [[ $(awk '{ print $22 }' "/proc/$owner_pid/stat") == "$bound_starttime" ]] || return 1
    [[ $(ps -o pgid= -p "$owner_pid" | tr -d ' ') == "$owner_pid" ]] || return 1
    [[ $(sha256sum "/proc/$owner_pid/cmdline" | awk '{ print $1 }') == "$expected_cmdline_sha256" ]]
}

launch_round() {
    local variant=$1 control=$2 round=$3 minimum=$4 remaining=$5
    local round_dir="$run_dir/$round"
    local owner_pgid owner_cmdline_sha256 owner_starttime status
    local stable=0
    check_quiet_host
    check_bindings
    check_available "$minimum"
    check_deadline "$remaining"
    check_disk
    mkdir "$round_dir"
    setsid "$script_dir/launch-controls.sh" --worker "$variant" "$control" "$round" "$round_dir" </dev/null &
    owner_pid=$!
    expected_cmdline_sha256=$(printf '/bin/bash\0%s\0--worker\0%s\0%s\0%s\0%s\0' \
        "$script_dir/launch-controls.sh" "$variant" "$control" "$round" "$round_dir" | sha256sum | awk '{ print $1 }')
    bound_starttime=
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
            local stop_deadline=$((SECONDS + 60))
            while same_bound_owner && ((SECONDS < stop_deadline)); do sleep 1; done
            if same_bound_owner; then kill -KILL -- "-$owner_pid" 2>/dev/null || true; fi
            echo 'identity failed; only captured identity signaled' > "$round_dir/launch-refusal.txt"
        else
            echo 'identity failed; no identity bound and no signal sent; timeout retained' > "$round_dir/launch-refusal.txt"
        fi
        exit 64
    fi
    owner_starttime=$bound_starttime
    cat > "$round_dir/identity.json" <<EOF
{"variant":"$variant","control":"$control","round":"$round","owner_pid":$owner_pid,"owner_pgid":$owner_pgid,"owner_starttime":$owner_starttime,"owner_cmdline_sha256":"$owner_cmdline_sha256"}
EOF
    set +e
    wait "$owner_pid"
    status=$?
    set -e
    [[ $status == 0 ]] || exit "$status"
    [[ $(du -sb "$run_dir" | awk '{ print $1 }') -le $MAX_ARTIFACT_BYTES ]]
}

launch_round untiled force untiled-force "$FORCE_MIN_AVAILABLE_KIB" 5640
launch_round tiled force tiled-force "$FORCE_MIN_AVAILABLE_KIB" 4680
launch_round untiled rhs untiled-rhs "$RHS_MIN_AVAILABLE_KIB" 3720
launch_round tiled rhs tiled-rhs "$RHS_MIN_AVAILABLE_KIB" 1860
date -u +'%Y-%m-%dT%H:%M:%SZ' > "$run_dir/run.end-utc"
