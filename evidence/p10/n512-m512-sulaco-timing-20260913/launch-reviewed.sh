#!/bin/bash
set -euo pipefail

readonly PRODUCTION_SOURCE=0843b8b18e6a096a0208e3d896e391c7b1b2f5e0
readonly TEST_SOURCE=9eba11f196a25f0843f0cbd0f4ed08c9f7ae4645
readonly HARNESS_SOURCE=f82df1b6d0e9a507d7bf11cd749dd94e19b08212
readonly BINARY_SHA256=635726f54b5078fa6a2093c812a4750ce904d67e517728f58e82a9b2f003d9f0
readonly SOURCE_MANIFEST_SHA256=0889f6fc2bd5572f443babe9bb38f8190f67884999bbe067bcb20ee8e3a32f8c
readonly INTERNAL_BYTES=207576840688
readonly OPERATIONAL_HEADROOM_BYTES=34359738368
readonly REQUIRED_AVAILABLE_BYTES=241936579056
readonly MIN_AVAILABLE_KIB=236266191
readonly AS_BYTES=274877906944
readonly MIN_DISK_AVAILABLE_BYTES=1073741824
readonly TIMEOUT_SECONDS=2700
readonly KILL_GRACE_SECONDS=60
readonly REVIEW_CLEANUP_RESERVE_SECONDS=300
readonly REQUIRED_REMAINING_SECONDS=3060
readonly DEADLINE_EPOCH=1789329174
readonly DEADLINE_UTC=2026-09-13T19:52:54Z

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
binary=$script_dir/p10-n512-m512-scratch-tail-timing
run_dir=$script_dir/run
owner_pid=
owner_starttime=
expected_cmdline_sha256=

same_owner() {
    [[ -n ${owner_pid-} && -n ${owner_starttime-} && -r /proc/$owner_pid/stat && -r /proc/$owner_pid/cmdline ]] || return 1
    [[ $owner_pid == "$(awk '{print $5}' /proc/$owner_pid/stat)" ]] || return 1
    [[ $owner_starttime == "$(awk '{print $22}' /proc/$owner_pid/stat)" ]] || return 1
    [[ $expected_cmdline_sha256 == "$(sha256sum /proc/$owner_pid/cmdline | awk '{print $1}')" ]]
}

cleanup_owned() {
    if same_owner; then
        /bin/kill -TERM -- "-$owner_pid" 2>/dev/null || true
        stop=$((SECONDS + KILL_GRACE_SECONDS))
        while same_owner && ((SECONDS < stop)); do sleep 1; done
        same_owner && /bin/kill -KILL -- "-$owner_pid" 2>/dev/null || true
    fi
}

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

[[ ${NSBU_LAUNCH_SULACO_N512_M512_SCRATCH_TAIL:-} == 1 ]] || { echo 'refused: exact launch variable required' >&2; exit 64; }
[[ $(hostname -s) == sulaco ]] || { echo 'refused: hostname is not sulaco' >&2; exit 64; }
[[ ! -e "$run_dir" ]] || { echo 'refused: create-new run directory required' >&2; exit 64; }
[[ $(sha256sum "$binary" | awk '{print $1}') == "$BINARY_SHA256" ]] || exit 65
[[ $(sha256sum "$script_dir/source-sha256.list" | awk '{print $1}') == "$SOURCE_MANIFEST_SHA256" ]] || exit 65
(cd "$script_dir" && sha256sum --check --status source-sha256.list) || exit 65
for proc_exe in /proc/[0-9]*/exe; do
    [[ -e $proc_exe ]] || continue
    executable=$(readlink "$proc_exe" 2>/dev/null || true)
    case ${executable##*/} in
        p10-*timing*|p10-*endpoint*|p10-*control*|nsbu-solver)
            echo "refused: active numerical executable $executable" >&2
            exit 66
            ;;
    esac
done
available_kib=$(awk '$1 == "MemAvailable:" {print $2}' /proc/meminfo)
[[ $available_kib =~ ^[0-9]+$ && $available_kib -ge $MIN_AVAILABLE_KIB ]] || { echo "refused: MemAvailable ${available_kib:-unknown} KiB below $MIN_AVAILABLE_KIB KiB" >&2; exit 66; }
available_disk_bytes=$(df -B1 --output=avail "$script_dir" | awk 'NR == 2 {print $1}')
[[ $available_disk_bytes =~ ^[0-9]+$ && $available_disk_bytes -ge $MIN_DISK_AVAILABLE_BYTES ]] || { echo "refused: disk available ${available_disk_bytes:-unknown} bytes below $MIN_DISK_AVAILABLE_BYTES" >&2; exit 66; }
remaining=$((DEADLINE_EPOCH - $(date -u +%s)))
((remaining >= REQUIRED_REMAINING_SECONDS)) || { echo "refused: need ${REQUIRED_REMAINING_SECONDS}s before $DEADLINE_UTC" >&2; exit 67; }

mkdir "$run_dir"
trap cleanup_owned EXIT HUP INT TERM
/usr/bin/setsid "$0" --worker "$run_dir" </dev/null &
owner_pid=$!
expected_cmdline_sha256=$(printf '/bin/bash\0%s\0--worker\0%s\0' "$0" "$run_dir" | sha256sum | awk '{print $1}')
stable=0
for _ in {1..200}; do
    if [[ -r /proc/$owner_pid/stat && -r /proc/$owner_pid/cmdline ]]; then
        observed_pgid=$(awk '{print $5}' /proc/$owner_pid/stat)
        observed_starttime=$(awk '{print $22}' /proc/$owner_pid/stat)
        observed_cmdline_sha256=$(sha256sum /proc/$owner_pid/cmdline | awk '{print $1}')
        if [[ $observed_pgid == "$owner_pid" && $observed_cmdline_sha256 == "$expected_cmdline_sha256" ]]; then
            [[ -z $owner_starttime || $owner_starttime == "$observed_starttime" ]] || stable=0
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
    echo 'refused: worker identity did not stabilize twice' > "$run_dir/launch-refusal.txt"
    exit 68
fi
cat > "$run_dir/launch-receipt.json" <<EOF
{"schema":"p10-n512-m512-sulaco-launch-receipt-v1","production_source":"$PRODUCTION_SOURCE","test_source":"$TEST_SOURCE","harness_source":"$HARNESS_SOURCE","binary_sha256":"$BINARY_SHA256","internal_resource_bytes":$INTERNAL_BYTES,"operational_headroom_bytes":$OPERATIONAL_HEADROOM_BYTES,"required_available_bytes":$REQUIRED_AVAILABLE_BYTES,"minimum_mem_available_kib":$MIN_AVAILABLE_KIB,"observed_mem_available_kib":$available_kib,"address_space_limit_bytes":$AS_BYTES,"disk_available_bytes":$available_disk_bytes,"remaining_seconds":$remaining,"owner_pid":$owner_pid,"owner_pgid":$owner_pid,"owner_starttime":$owner_starttime,"owner_cmdline_sha256":"$expected_cmdline_sha256"}
EOF
set +e
wait "$owner_pid"
status=$?
set -e
printf '%s\n' "$status" > "$run_dir/launcher.status"
owner_pid=
exit "$status"
