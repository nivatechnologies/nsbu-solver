#!/bin/bash
set -euo pipefail
readonly BINARY_SHA256=da096a76436caaf05640a6417716913e0f272b10cfc92007d23bb3ff517b23b2
readonly PLAN_BYTES=207626205968
readonly HEADROOM_BYTES=34359738368
readonly MIN_AVAILABLE_KIB=236314399
readonly AS_KIB=268435456
readonly TIMEOUT_SECONDS=1800
readonly KILL_GRACE_SECONDS=60

script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
repo=$(CDPATH= cd -- "$script_dir/../../.." && pwd)
binary=$script_dir/harness/target/release/p10-n512-m512-parallel-fft-timing
run_dir=$script_dir/raw/parallel-attempt-20260913
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
    ulimit -v "$AS_KIB"
    printf '%s\n' "$((AS_KIB * 1024))" > "$run_dir/run.rlimit-as-bytes"
    date -u +%FT%TZ > "$run_dir/run.start-utc"
    set +e
    NSBU_RUN_N512_M512_PARALLEL_FFT_ONE_ATTEMPT=1 /usr/bin/time -v -o "$run_dir/run.time" \
        /usr/bin/timeout --foreground --signal=TERM --kill-after="${KILL_GRACE_SECONDS}s" \
        "$TIMEOUT_SECONDS" "$binary" run-parallel "$run_dir/result.json" \
        > "$run_dir/run.stdout" 2> "$run_dir/run.stderr"
    status=$?
    set -e
    printf '%s\n' "$status" > "$run_dir/run.status"
    date -u +%FT%TZ > "$run_dir/run.end-utc"
    exit "$status"
fi

[[ ${NSBU_LAUNCH_N512_M512_PARALLEL_FFT:-} == 1 ]] || { echo 'refused: exact launch variable required' >&2; exit 64; }
[[ $(hostname -s) == baccus ]] || { echo 'refused: wrong host' >&2; exit 64; }
[[ ! -e $run_dir ]] || { echo 'refused: create-new run directory required' >&2; exit 64; }
[[ $(sha256sum "$binary" | awk '{print $1}') == "$BINARY_SHA256" ]] || exit 65
(cd "$repo" && sha256sum --check --status "$script_dir/prepared/source-sha256.list") || exit 65
for proc_exe in /proc/[0-9]*/exe; do
    [[ -e $proc_exe ]] || continue
    executable=$(readlink "$proc_exe" 2>/dev/null || true)
    [[ $executable != "$binary" ]] || { echo 'refused: exact candidate already active' >&2; exit 66; }
done
available_kib=$(awk '$1=="MemAvailable:" {print $2}' /proc/meminfo)
[[ $available_kib =~ ^[0-9]+$ && $available_kib -ge $MIN_AVAILABLE_KIB ]] || exit 66
available_disk_bytes=$(df -B1 --output=avail "$script_dir" | awk 'NR==2 {print $1}')
[[ $available_disk_bytes =~ ^[0-9]+$ && $available_disk_bytes -ge 1073741824 ]] || exit 66
mkdir -p "$script_dir/raw"
mkdir "$run_dir"
uptime > "$run_dir/host-load.txt"
ps -eo pid,ppid,pgid,psr,pcpu,pmem,rss,stat,args > "$run_dir/processes-before.txt"
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
[[ $stable -ge 2 ]] || { echo 'refused: worker identity did not stabilize twice' > "$run_dir/launch-refusal.txt"; exit 68; }
cat > "$run_dir/launch-receipt.json" <<EOF
{"schema":"p10-n512-m512-parallel-fft-launch-receipt-v1","library_source_commit":"477c418d30d9f2c8b117ae05240fc5596ecbd33b","harness_source_commit":"2284c561184f9dff06a4bd8e339a7d7594d0ab88","binary_sha256":"$BINARY_SHA256","plan_bytes":$PLAN_BYTES,"operational_headroom_bytes":$HEADROOM_BYTES,"minimum_mem_available_kib":$MIN_AVAILABLE_KIB,"observed_mem_available_kib":$available_kib,"disk_available_bytes":$available_disk_bytes,"timeout_seconds":$TIMEOUT_SECONDS,"kill_grace_seconds":$KILL_GRACE_SECONDS,"owner_pid":$owner_pid,"owner_pgid":$owner_pid,"owner_starttime":$owner_starttime,"owner_cmdline_sha256":"$expected_cmdline_sha256"}
EOF
set +e
wait "$owner_pid"
status=$?
set -e
printf '%s\n' "$status" > "$run_dir/launcher.status"
owner_pid=
exit "$status"
