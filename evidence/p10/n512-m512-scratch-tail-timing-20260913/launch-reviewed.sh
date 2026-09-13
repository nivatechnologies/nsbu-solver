#!/bin/bash
set -euo pipefail

readonly PRODUCTION_SOURCE=0843b8b18e6a096a0208e3d896e391c7b1b2f5e0
readonly TEST_SOURCE=9eba11f196a25f0843f0cbd0f4ed08c9f7ae4645
readonly HARNESS_SOURCE=f82df1b6d0e9a507d7bf11cd749dd94e19b08212
readonly BINARY_SHA256=635726f54b5078fa6a2093c812a4750ce904d67e517728f58e82a9b2f003d9f0
readonly INTERNAL_BYTES=207576840688
readonly AS_BYTES=274877906944
readonly MIN_AVAILABLE_KIB=285212672
readonly MIN_DISK_AVAILABLE_BYTES=1073741824
readonly TIMEOUT_SECONDS=2700
readonly KILL_GRACE_SECONDS=60
readonly REVIEW_CLEANUP_RESERVE_SECONDS=300
readonly REQUIRED_REMAINING_SECONDS=3060
readonly DEADLINE_EPOCH=1789329174
readonly DEADLINE_UTC=2026-09-13T19:52:54Z

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo=$(CDPATH= cd -- "$script_dir/../../.." && pwd)
binary=$script_dir/harness/target/release/p10-n512-m512-scratch-tail-timing
run_dir=$script_dir/raw/actual-attempt-20260913

if [[ ${1-} == --worker ]]; then
    shift
    [[ $# == 1 && $1 == "$run_dir" ]] || exit 64
    ulimit -v 268435456
    printf '%s\n' "$AS_BYTES" > "$run_dir/run.rlimit-as-bytes"
    date -u +'%Y-%m-%dT%H:%M:%SZ' > "$run_dir/run.start-utc"
    set +e
    NSBU_RUN_N512_M512_ONE_ATTEMPT=1 /usr/bin/time -v -o "$run_dir/run.time" \
        /usr/bin/timeout --foreground --signal=TERM --kill-after="${KILL_GRACE_SECONDS}s" \
        "$TIMEOUT_SECONDS" "$binary" run "$run_dir/result.json" \
        > "$run_dir/run.stdout" 2> "$run_dir/run.stderr"
    status=$?
    set -e
    printf '%s\n' "$status" > "$run_dir/run.status"
    date -u +'%Y-%m-%dT%H:%M:%SZ' > "$run_dir/run.end-utc"
    exit "$status"
fi

[[ ${NSBU_LAUNCH_N512_M512_SCRATCH_TAIL:-} == 1 ]] || {
    echo 'refused: exact root launch variable required' >&2
    exit 64
}
[[ ${PRIOR_N512_M768_RELEASE_CONFIRMED:-} == 1 ]] || {
    echo 'refused: prior N512/M768 owner release confirmation required' >&2
    exit 64
}
for command_file in /proc/[0-9]*/cmdline; do
    [[ -r $command_file ]] || continue
    command=$(tr '\0' ' ' < "$command_file" 2>/dev/null || true)
    case "$command" in
        *p10-n512-m768-scratch-tail-timing-20260913*|*p10-n512-m768-scratch-tail-timing*)
            echo "refused: active prior N512/M768 timing owner: $command" >&2
            exit 64
            ;;
    esac
done
[[ ! -e "$run_dir" ]] || { echo 'refused: create-new run directory required' >&2; exit 64; }
[[ $(sha256sum "$binary" | awk '{print $1}') == "$BINARY_SHA256" ]] || exit 65
git -C "$repo" merge-base --is-ancestor "$TEST_SOURCE" HEAD
git -C "$repo" diff --quiet "$TEST_SOURCE" -- crates/nsbu-solver
git -C "$repo" diff --quiet -- crates/nsbu-solver
(cd "$repo" && sha256sum --check --status "$script_dir/prepared/source-sha256.list")
available_kib=$(awk '$1 == "MemAvailable:" {print $2}' /proc/meminfo)
[[ $available_kib =~ ^[0-9]+$ && $available_kib -ge $MIN_AVAILABLE_KIB ]] || {
    echo "refused: MemAvailable ${available_kib:-unknown} KiB below $MIN_AVAILABLE_KIB KiB" >&2
    exit 66
}
available_disk_bytes=$(df -B1 --output=avail "$script_dir" | awk 'NR == 2 {print $1}')
[[ $available_disk_bytes =~ ^[0-9]+$ && $available_disk_bytes -ge $MIN_DISK_AVAILABLE_BYTES ]] || {
    echo "refused: disk available ${available_disk_bytes:-unknown} bytes below $MIN_DISK_AVAILABLE_BYTES" >&2
    exit 66
}
remaining=$((DEADLINE_EPOCH - $(date -u +%s)))
((remaining >= REQUIRED_REMAINING_SECONDS)) || {
    echo "refused: need ${REQUIRED_REMAINING_SECONDS}s through review cleanup reserve before $DEADLINE_UTC" >&2
    exit 67
}

mkdir -p "$script_dir/raw"
mkdir "$run_dir"
/usr/bin/setsid "$script_dir/launch-reviewed.sh" --worker "$run_dir" </dev/null &
owner_pid=$!
expected_cmdline_sha256=$(
    printf '/bin/bash\0%s\0--worker\0%s\0' "$script_dir/launch-reviewed.sh" "$run_dir" |
        sha256sum | awk '{print $1}'
)
same_owner() {
    [[ -n ${owner_starttime-} && -r /proc/$owner_pid/stat && -r /proc/$owner_pid/cmdline ]] || return 1
    [[ $owner_pid == "$(awk '{print $5}' /proc/$owner_pid/stat)" ]] || return 1
    [[ $owner_starttime == "$(awk '{print $22}' /proc/$owner_pid/stat)" ]] || return 1
    [[ $expected_cmdline_sha256 == "$(sha256sum /proc/$owner_pid/cmdline | awk '{print $1}')" ]]
}
stable=0
owner_starttime=
for _ in {1..200}; do
    if [[ -r /proc/$owner_pid/stat && -r /proc/$owner_pid/cmdline ]]; then
        observed_pgid=$(awk '{print $5}' /proc/$owner_pid/stat)
        observed_starttime=$(awk '{print $22}' /proc/$owner_pid/stat)
        observed_cmdline_sha256=$(sha256sum /proc/$owner_pid/cmdline | awk '{print $1}')
        if [[ $observed_pgid == "$owner_pid" && $observed_cmdline_sha256 == "$expected_cmdline_sha256" ]]; then
            if [[ -n $owner_starttime && $owner_starttime != "$observed_starttime" ]]; then
                stable=0
            fi
            owner_starttime=$observed_starttime
            stable=$((stable + 1))
            [[ $stable -ge 2 ]] && break
        else
            stable=0
            owner_starttime=
        fi
    fi
    sleep 0.01
done
if [[ $stable -lt 2 ]]; then
    if same_owner; then
        /bin/kill -TERM -- "-$owner_pid" 2>/dev/null || true
        stop=$((SECONDS + KILL_GRACE_SECONDS))
        while same_owner && ((SECONDS < stop)); do sleep 1; done
        same_owner && /bin/kill -KILL -- "-$owner_pid" 2>/dev/null || true
    fi
    echo 'refused: worker identity did not stabilize twice' > "$run_dir/launch-refusal.txt"
    exit 68
fi
cat > "$run_dir/launch-receipt.json" <<EOF
{
  "schema": "p10-n512-m512-scratch-tail-launch-receipt-v1",
  "production_source_commit": "$PRODUCTION_SOURCE",
  "test_source_commit": "$TEST_SOURCE",
  "harness_source_commit": "$HARNESS_SOURCE",
  "binary_sha256": "$BINARY_SHA256",
  "internal_resource_bytes": $INTERNAL_BYTES,
  "address_space_limit_bytes": $AS_BYTES,
  "mem_available_kib": $available_kib,
  "disk_available_bytes": $available_disk_bytes,
  "minimum_disk_available_bytes": $MIN_DISK_AVAILABLE_BYTES,
  "timeout_seconds": $TIMEOUT_SECONDS,
  "kill_grace_seconds": $KILL_GRACE_SECONDS,
  "review_cleanup_reserve_seconds": $REVIEW_CLEANUP_RESERVE_SECONDS,
  "campaign_deadline_epoch": $DEADLINE_EPOCH,
  "owner_pid": $owner_pid,
  "owner_pgid": $owner_pid,
  "owner_starttime": $owner_starttime,
  "owner_cmdline_sha256": "$expected_cmdline_sha256",
  "run_dir": "$run_dir"
}
EOF
printf 'launched run_dir=%s owner_pid=%s process_group=%s starttime=%s\n' \
    "$run_dir" "$owner_pid" "$owner_pid" "$owner_starttime"
