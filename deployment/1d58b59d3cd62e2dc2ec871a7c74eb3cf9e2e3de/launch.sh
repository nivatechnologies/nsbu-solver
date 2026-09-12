#!/bin/sh
set -eu

[ "$#" -eq 1 ] || { echo "usage: $0 ABSENT_RUN_DIRECTORY" >&2; exit 64; }
run_dir=$1
[ ! -e "$run_dir" ] || { echo "run directory already exists" >&2; exit 65; }
case "$run_dir" in /*) ;; *) echo "absolute run directory required" >&2; exit 64;; esac
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
bin="$script_dir/p10-avx-ho-pilot"
preflight="$script_dir/preflight.stdout"
watchdog="$script_dir/pgid-watchdog-v2.sh"

verify() {
    actual=$(sha256sum "$1" | awk '{print $1}')
    [ "$actual" = "$2" ] || { echo "hash mismatch: $1" >&2; exit 66; }
}
verify "$bin" 252826b508ed4418f9a90f9bcc21d25a911d47d3558e4ad1c87fbeeeb9a52729
verify "$preflight" c4f90755b04b54769815126d6073c5cbc167b359ca0f65f409e0522296711367
verify "$watchdog" 4b65e13b74bd7a32237044b5467d4f223c8dc3e6e35d6e8d3498e1938fa53e4b

mkdir "$run_dir"
"$bin" preflight unused > "$run_dir/fresh-preflight.stdout"
verify "$run_dir/fresh-preflight.stdout" c4f90755b04b54769815126d6073c5cbc167b359ca0f65f409e0522296711367
mem_kib=$(awk '$1 == "MemAvailable:" {print $2}' /proc/meminfo)
mem_bytes=$((mem_kib * 1024))
[ "$mem_bytes" -ge 274877906944 ] || { echo "live admission refused: MemAvailable=$mem_bytes" | tee "$run_dir/refusal.txt" >&2; exit 67; }
{
    date -u +started_at=%Y-%m-%dT%H:%M:%SZ
    echo memavailable_bytes="$mem_bytes"
    echo required_floor_bytes=274877906944
    echo reservation_bytes=198987813712
    echo cap_bytes=240518168576
    echo label=contended-local-rest-timing-only
    for protected in 430766 842148; do
        if [ -r "/proc/$protected/cmdline" ]; then
            printf 'protected_pid=%s cmdline_sha256=' "$protected"
            sha256sum "/proc/$protected/cmdline" | awk '{print $1}'
        else
            echo protected_pid="$protected" status=already_exited
        fi
    done
} > "$run_dir/admission.txt"

output="$run_dir/output"
setsid /usr/bin/time -v -o "$run_dir/time.txt" "$bin" run "$output" > "$run_dir/stdout" 2> "$run_dir/stderr" &
wrapper_pid=$!
process_group=$wrapper_pid
solver_pid=
tries=0
while [ "$tries" -lt 100 ]; do
    solver_pid=$(pgrep -P "$wrapper_pid" | head -1 || true)
    [ -n "$solver_pid" ] && break
    sleep 0.1
    tries=$((tries + 1))
done
[ -n "$solver_pid" ] || { kill -TERM -- "-$process_group" 2>/dev/null || true; echo "solver child not found" >&2; exit 68; }
actual_exe_sha=$(sha256sum "/proc/$solver_pid/exe" | awk '{print $1}')
[ "$actual_exe_sha" = 252826b508ed4418f9a90f9bcc21d25a911d47d3558e4ad1c87fbeeeb9a52729 ] || { kill -TERM -- "-$process_group" 2>/dev/null || true; echo "solver executable mismatch" >&2; exit 69; }
stat=$(cat "/proc/$solver_pid/stat")
rest=${stat##*) }
set -- $rest
[ "$3" = "$process_group" ] || { kill -TERM -- "-$process_group" 2>/dev/null || true; echo "solver process group mismatch" >&2; exit 69; }
starttime=${20}
cmdline_sha=$(sha256sum "/proc/$solver_pid/cmdline" | awk '{print $1}')
deadline_epoch=$(($(date +%s) + 1200))
setsid "$watchdog" "$solver_pid" "$process_group" "$starttime" "$cmdline_sha" "$deadline_epoch" "$run_dir/watchdog.log" > "$run_dir/watchdog.stdout" 2>&1 &
watchdog_pid=$!
{
    echo wrapper_pid="$wrapper_pid"
    echo solver_pid="$solver_pid"
    echo process_group="$process_group"
    echo solver_starttime="$starttime"
    echo solver_cmdline_sha256="$cmdline_sha"
    echo watchdog_pid="$watchdog_pid"
    echo deadline_epoch="$deadline_epoch"
} >> "$run_dir/admission.txt"
set +e
wait "$wrapper_pid"
status=$?
set -e
wait "$watchdog_pid" || true
printf 'exit_status=%s\nfinished_at=%s\n' "$status" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$run_dir/exit.txt"
exit "$status"
