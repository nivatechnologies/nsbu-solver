#!/bin/sh
set -eu
[ "$#" -eq 1 ] || { echo "usage: $0 ABSENT_ABSOLUTE_RUN_DIRECTORY" >&2; exit 64; }
run_dir=$1
case "$run_dir" in /*) ;; *) echo "absolute run directory required" >&2; exit 64;; esac
[ ! -e "$run_dir" ] || { echo "run directory already exists" >&2; exit 65; }
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
bin="$script_dir/p10-avx-n384-piecewise-ho-cadv33"
preflight="$script_dir/preflight.stdout"
watchdog="$script_dir/pgid-watchdog-v2.sh"
verify() { actual=$(sha256sum "$1" | awk '{print $1}'); [ "$actual" = "$2" ] || { echo "hash mismatch: $1" >&2; exit 66; }; }
verify "$bin" 9112ad80007147550a81df1ac870af6e09b0a2dae9f28a6510f96fbf861a86a0
verify "$preflight" 9ec05aa954479d101c2facafc9e502ffd09174cbdc1d7a4f3502e7a85278dc9f
verify "$watchdog" 4b65e13b74bd7a32237044b5467d4f223c8dc3e6e35d6e8d3498e1938fa53e4b
[ ! -e /proc/842148/stat ] || { echo "matched N256 remains live" >&2; exit 67; }
now=$(date +%s)
[ "$now" -le 1789259510 ] || { echo "latest operational start exceeded" >&2; exit 68; }
mem_kib=$(awk '$1 == "MemAvailable:" {print $2}' /proc/meminfo)
mem_bytes=$((mem_kib * 1024))
[ "$mem_bytes" -ge 240518168576 ] || { echo "memory floor refused: $mem_bytes" >&2; exit 69; }
disk_bytes=$(df -B1 --output=avail "$(dirname "$run_dir")" | tail -1 | tr -d ' ')
[ "$disk_bytes" -ge 138512695296 ] || { echo "disk floor refused: $disk_bytes" >&2; exit 70; }
mkdir -p "$(dirname "$run_dir")"
mkdir "$run_dir"
"$bin" preflight unused > "$run_dir/fresh-preflight.stdout"
verify "$run_dir/fresh-preflight.stdout" 9ec05aa954479d101c2facafc9e502ffd09174cbdc1d7a4f3502e7a85278dc9f
{
 date -u +started_at=%Y-%m-%dT%H:%M:%SZ
 echo memavailable_bytes="$mem_bytes"
 echo disk_available_bytes="$disk_bytes"
 echo reservation_bytes=198987813712
 echo execution_cap_bytes=206158430208
 echo artifact_cap_bytes=137438953472
 echo watchdog_term_epoch=1789282740
 echo numerical_hard_stop_epoch=1789282800
} > "$run_dir/admission.txt"
output="$run_dir/output"
remaining_seconds=$((1789282740 - now))
setsid /usr/bin/time -v -o "$run_dir/time.txt" timeout --foreground --signal=TERM --kill-after=60 "$remaining_seconds" "$bin" run "$output" > "$run_dir/stdout" 2> "$run_dir/stderr" &
wrapper_pid=$!
process_group=$wrapper_pid
timeout_pid=
solver_pid=
tries=0
while [ "$tries" -lt 100 ]; do timeout_pid=$(pgrep -P "$wrapper_pid" | head -1 || true); [ -n "$timeout_pid" ] && solver_pid=$(pgrep -P "$timeout_pid" | head -1 || true); [ -n "$solver_pid" ] && break; sleep 0.1; tries=$((tries+1)); done
[ -n "$solver_pid" ] || { kill -TERM -- "-$process_group" 2>/dev/null || true; exit 71; }
actual_exe_sha=$(sha256sum "/proc/$solver_pid/exe" | awk '{print $1}')
[ "$actual_exe_sha" = 9112ad80007147550a81df1ac870af6e09b0a2dae9f28a6510f96fbf861a86a0 ] || { kill -TERM -- "-$process_group" 2>/dev/null || true; exit 72; }
stat=$(cat "/proc/$solver_pid/stat"); rest=${stat##*) }; set -- $rest
[ "$3" = "$process_group" ] || { kill -TERM -- "-$process_group" 2>/dev/null || true; exit 72; }
starttime=${20}; cmdline_sha=$(sha256sum "/proc/$solver_pid/cmdline" | awk '{print $1}')
setsid "$watchdog" "$solver_pid" "$process_group" "$starttime" "$cmdline_sha" 1789282740 "$run_dir/watchdog.log" > "$run_dir/watchdog.stdout" 2>&1 &
watchdog_pid=$!
expected_start="started leader_pid=$solver_pid process_group=$process_group starttime=$starttime cmdline_sha256=$cmdline_sha deadline_epoch=1789282740"
tries=0
while [ "$tries" -lt 100 ]; do grep -F "$expected_start" "$run_dir/watchdog.log" >/dev/null 2>&1 && break; kill -0 "$watchdog_pid" 2>/dev/null || { kill -TERM -- "-$process_group" 2>/dev/null || true; wait "$wrapper_pid" || true; exit 73; }; sleep 0.1; tries=$((tries+1)); done
grep -F "$expected_start" "$run_dir/watchdog.log" >/dev/null 2>&1 || { kill -TERM -- "-$process_group" 2>/dev/null || true; wait "$wrapper_pid" || true; kill "$watchdog_pid" 2>/dev/null || true; exit 73; }
{
 echo time_pid="$wrapper_pid"; echo timeout_pid="$timeout_pid"; echo solver_pid="$solver_pid"; echo process_group="$process_group"; echo solver_starttime="$starttime"; echo solver_cmdline_sha256="$cmdline_sha"; echo watchdog_pid="$watchdog_pid"; echo watchdog_handshake=confirmed;
} >> "$run_dir/admission.txt"
set +e; wait "$wrapper_pid"; status=$?; set -e
wait "$watchdog_pid" || true
printf 'exit_status=%s\nfinished_at=%s\n' "$status" "$(date -u +%Y-%m-%dT%H:%M:%SZ)" > "$run_dir/exit.txt"
exit "$status"
