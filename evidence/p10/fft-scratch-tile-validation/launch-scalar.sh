#!/bin/bash
set -euo pipefail

readonly PROTOTYPE_SOURCE=9eba11f196a25f0843f0cbd0f4ed08c9f7ae4645
readonly UNTILED_SOURCE=f3960e157f26f37fa9a9fe808a4a596395164706
readonly TILED_BINARY_SHA256=631067278e605e0901539069387fd647d188c26e223ce6628c7aab57b8ff4c43
readonly UNTILED_BINARY_SHA256=003969a932f7cc794db8030c99dc50aca6256f48307c34fdd80eedf7d9b2b984
readonly AS_BYTES=25769803776
readonly MIN_AVAILABLE_KIB=67108864
readonly MAX_ARTIFACT_BYTES=4194304
readonly MIN_DISK_AVAILABLE_BYTES=8388608
readonly CAMPAIGN_DEADLINE_UTC=2026-09-13T19:52:54Z
readonly CAMPAIGN_DEADLINE_EPOCH=1789329174

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo=$(CDPATH= cd -- "$script_dir/../../.." && pwd)
baseline_repo=/mnt/niva-array/nsbu-solver/work/p10-fft-transverse-untiled-baseline-20260913
tiled_binary="$script_dir/harness/target/release/p10-fft-transverse-tile-benchmark"
untiled_binary="$baseline_repo/evidence/p10/fft-transverse-tile-prototype/harness/target/release/p10-fft-transverse-tile-benchmark"
run_dir="$script_dir/raw-scalar/ab-20260913"

if [[ ${1-} == --worker ]]; then
    shift
    [[ $# == 3 ]] || exit 64
    variant=$1
    round=$2
    round_dir=$3
    [[ $round_dir == "$run_dir/$round" && -d $round_dir ]] || exit 64
    case $variant in
        tiled) binary=$tiled_binary ;;
        untiled) binary=$untiled_binary ;;
        *) exit 64 ;;
    esac
    ulimit -v 25165824
    printf '%s\n' "$AS_BYTES" > "$round_dir/run.rlimit-as-bytes"
    date -u +'%Y-%m-%dT%H:%M:%SZ' > "$round_dir/run.start-utc"
    set +e
    /usr/bin/time -v -o "$round_dir/run.time" \
        timeout --foreground --signal=TERM --kill-after=60s 600s \
        "$binary" 768 1 "$round_dir/unused-word-output" --hash-only \
        > "$round_dir/run.stdout" 2> "$round_dir/run.stderr"
    status=$?
    set -e
    printf '%s\n' "$status" > "$round_dir/run.status"
    date -u +'%Y-%m-%dT%H:%M:%SZ' > "$round_dir/run.end-utc"
    exit "$status"
fi

[[ ${SCALAR_LAUNCH_AUTHORIZED-} == 1 ]] || {
    echo 'refusing launch without SCALAR_LAUNCH_AUTHORIZED=1' >&2
    exit 64
}

check_endpoint() {
    if pgrep -f '[p]rojection-n512|[p]10-n512-m768-one-attempt|[p]10-w3-768-controls (force|rhs)|[p]10-fft-transverse-tile-benchmark 768' >/dev/null; then
        echo 'refusing launch while a conflicting numerical command remains' >&2
        exit 64
    fi
}

check_bindings() {
    [[ $(git -C "$repo" rev-parse --verify "$PROTOTYPE_SOURCE") == "$PROTOTYPE_SOURCE"* ]]
    git -C "$repo" diff --quiet "$PROTOTYPE_SOURCE" -- crates/nsbu-solver
    git -C "$repo" diff --quiet -- \
        crates/nsbu-solver evidence/p10/fft-scratch-tile-validation/harness
    [[ $(git -C "$baseline_repo" rev-parse HEAD) == "$UNTILED_SOURCE" ]]
    git -C "$baseline_repo" diff --quiet -- crates/nsbu-solver
    for source in Cargo.toml Cargo.lock src/main.rs src/bin/anisotropic.rs; do
        cmp -s "$script_dir/harness/$source" \
            "$baseline_repo/evidence/p10/fft-transverse-tile-prototype/harness/$source"
    done
    [[ $(sha256sum "$tiled_binary" | awk '{ print $1 }') == "$TILED_BINARY_SHA256" ]]
    [[ $(sha256sum "$untiled_binary" | awk '{ print $1 }') == "$UNTILED_BINARY_SHA256" ]]
}

check_available() {
    available_kib=$(awk '$1 == "MemAvailable:" { print $2 }' /proc/meminfo)
    [[ $available_kib =~ ^[0-9]+$ && $available_kib -ge $MIN_AVAILABLE_KIB ]] || {
        echo "refusing launch: MemAvailable ${available_kib:-unknown} KiB below $MIN_AVAILABLE_KIB KiB" >&2
        exit 64
    }
}

check_deadline() {
    local remaining=$1
    local now_epoch
    now_epoch=$(date -u +%s)
    ((now_epoch + remaining * 660 + 300 <= CAMPAIGN_DEADLINE_EPOCH)) || {
        echo "refusing launch: remaining rounds plus preparation exceed $CAMPAIGN_DEADLINE_UTC" >&2
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

check_endpoint
check_bindings
check_available
check_deadline 4
check_disk
if [[ ${1-} == --preflight-only ]]; then
    echo "preflight passed MemAvailable=${available_kib}KiB deadline=$CAMPAIGN_DEADLINE_UTC"
    exit 0
fi
[[ $# == 0 ]] || exit 64
mkdir -p "$script_dir/raw-scalar"
mkdir "$run_dir"
cat > "$run_dir/launch-receipt.json" <<EOF
{
  "schema": "p10-fft-scratch-tile-n768-launch-receipt-v1",
  "prototype_source": "$PROTOTYPE_SOURCE",
  "untiled_source": "$UNTILED_SOURCE",
  "tiled_binary_sha256": "$TILED_BINARY_SHA256",
  "untiled_binary_sha256": "$UNTILED_BINARY_SHA256",
  "address_space_limit_bytes": $AS_BYTES,
  "tiled_accounted_payload_workspace_catalog_bytes": 14543923912,
  "untiled_accounted_payload_workspace_catalog_bytes": 14543825608,
  "harness_runtime_overhead_allowance_bytes": 1048576,
  "tiled_declared_peak_bytes": 14544972488,
  "untiled_declared_peak_bytes": 14544874184,
  "prelaunch_mem_available_kib": $available_kib,
  "minimum_mem_available_bytes": 68719476736,
  "maximum_artifact_bytes": $MAX_ARTIFACT_BYTES,
  "prelaunch_disk_available_bytes": $disk_available_bytes,
  "minimum_disk_available_bytes": $MIN_DISK_AVAILABLE_BYTES,
  "campaign_deadline_utc": "$CAMPAIGN_DEADLINE_UTC",
  "campaign_deadline_epoch": $CAMPAIGN_DEADLINE_EPOCH,
  "round_order": ["untiled-a", "tiled-a", "tiled-b", "untiled-b"]
}
EOF

same_bound_owner() {
    [[ -n ${bound_starttime-} && -r /proc/$owner_pid/stat && -r /proc/$owner_pid/cmdline ]] || return 1
    [[ $(awk '{ print $22 }' "/proc/$owner_pid/stat") == "$bound_starttime" ]] || return 1
    [[ $(ps -o pgid= -p "$owner_pid" | tr -d ' ') == "$owner_pid" ]] || return 1
    [[ $(sha256sum "/proc/$owner_pid/cmdline" | awk '{ print $1 }') == "$expected_cmdline_sha256" ]]
}

launch_round() {
    local variant=$1
    local round=$2
    local remaining=$3
    local round_dir="$run_dir/$round"
    local owner_pgid owner_cmdline_sha256 owner_starttime status
    local stable=0
    check_endpoint
    check_bindings
    check_available
    check_deadline "$remaining"
    check_disk
    mkdir "$round_dir"
    setsid "$script_dir/launch-scalar.sh" --worker "$variant" "$round" "$round_dir" </dev/null &
    owner_pid=$!
    expected_cmdline_sha256=$(
        printf '/bin/bash\0%s\0--worker\0%s\0%s\0%s\0' \
            "$script_dir/launch-scalar.sh" "$variant" "$round" "$round_dir" | sha256sum | awk '{ print $1 }'
    )
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
{"variant":"$variant","round":"$round","owner_pid":$owner_pid,"owner_pgid":$owner_pgid,"owner_starttime":$owner_starttime,"owner_cmdline_sha256":"$owner_cmdline_sha256"}
EOF
    set +e
    wait "$owner_pid"
    status=$?
    set -e
    [[ $status == 0 ]] || exit "$status"
    [[ $(du -sb "$run_dir" | awk '{ print $1 }') -le $MAX_ARTIFACT_BYTES ]]
}

launch_round untiled untiled-a 4
launch_round tiled tiled-a 3
launch_round tiled tiled-b 2
launch_round untiled untiled-b 1
date -u +'%Y-%m-%dT%H:%M:%SZ' > "$run_dir/run.end-utc"
