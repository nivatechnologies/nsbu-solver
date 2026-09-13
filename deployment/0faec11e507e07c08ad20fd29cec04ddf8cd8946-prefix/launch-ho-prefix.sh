#!/bin/sh
set -eu
[ "$#" -eq 1 ] || { echo "usage: $0 ABSENT_ABSOLUTE_RUN_DIRECTORY" >&2; exit 64; }
run_dir=$1
case "$run_dir" in /*) ;; *) echo "absolute run directory required" >&2; exit 64;; esac
[ ! -e "$run_dir" ] || { echo "run directory already exists" >&2; exit 65; }
script_dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
bin="$script_dir/p10-avx-n384-piecewise-ho-cadv33"
preflight="$script_dir/preflight.stdout"
watchdog="$script_dir/pgid-watchdog-v3.sh"
plan="$script_dir/frozen-plan-ho-prefix.json"
verify() { actual=$(sha256sum "$1" | awk '{print $1}'); [ "$actual" = "$2" ] || { echo "hash mismatch: $1" >&2; exit 66; }; }
verify "$bin" 9112ad80007147550a81df1ac870af6e09b0a2dae9f28a6510f96fbf861a86a0
verify "$preflight" 9ec05aa954479d101c2facafc9e502ffd09174cbdc1d7a4f3502e7a85278dc9f
verify "$watchdog" 0628e9d65dd65f7738cbaf3bffe720dc357faba1d116d45ec1cdf2c60507ab01
verify "$plan" ef337eb28924a335a78dbfc1c710ca40e8b507b10b96687bc4197971e411b60c
[ ! -e /proc/842148/stat ] || { echo "matched N256 remains live" >&2; exit 67; }
now=$(date +%s)
[ "$now" -le 1789267658 ] || { echo "latest operational start exceeded" >&2; exit 68; }
mem_kib=$(awk '$1 == "MemAvailable:" {print $2}' /proc/meminfo)
mem_bytes=$((mem_kib * 1024))
[ "$mem_bytes" -ge 240518168576 ] || { echo "memory floor refused: $mem_bytes" >&2; exit 69; }
run_parent=$(dirname "$run_dir")
mkdir -p "$run_parent"
disk_bytes=$(df -B1 --output=avail "$run_parent" | tail -1 | tr -d ' ')
[ "$disk_bytes" -ge 138512695296 ] || { echo "disk floor refused: $disk_bytes" >&2; exit 70; }
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
setsid /usr/bin/time -v -o "$run_dir/time.txt" /usr/bin/timeout --foreground --signal=TERM --kill-after=60 "$remaining_seconds" "$bin" run "$output" > "$run_dir/stdout" 2> "$run_dir/stderr" &
wrapper_pid=$!
process_group=$wrapper_pid
timeout_pid=
solver_pid=
tries=0
while [ "$tries" -lt 100 ]; do timeout_pid=$(pgrep -P "$wrapper_pid" | head -1 || true); [ -n "$timeout_pid" ] && solver_pid=$(pgrep -P "$timeout_pid" | head -1 || true); [ -n "$solver_pid" ] && break; sleep 0.1; tries=$((tries+1)); done
[ -n "$solver_pid" ] || { /bin/kill -TERM -- "-$process_group" 2>/dev/null || true; exit 71; }
expected_cmdline_sha=$(printf '%s\0%s\0%s\0' "$bin" run "$output" | sha256sum | awk '{print $1}')
identity_snapshot() {
 [ -r "/proc/$solver_pid/stat" ] && [ -r "/proc/$solver_pid/cmdline" ] || return 1
 actual_exe_sha=$(sha256sum "/proc/$solver_pid/exe" 2>/dev/null | awk '{print $1}') || return 1
 [ "$actual_exe_sha" = 9112ad80007147550a81df1ac870af6e09b0a2dae9f28a6510f96fbf861a86a0 ] || return 1
 stat=$(cat "/proc/$solver_pid/stat") || return 1
 rest=${stat##*) }; set -- $rest
 [ "$1" != Z ] && [ "$3" = "$process_group" ] || return 1
 actual_cmdline_sha=$(sha256sum "/proc/$solver_pid/cmdline" | awk '{print $1}') || return 1
 [ "$actual_cmdline_sha" = "$expected_cmdline_sha" ] || return 1
 printf '%s:%s:%s\n' "${20}" "$3" "$actual_cmdline_sha"
}
previous=
tries=0
stable=
while [ "$tries" -lt 100 ]; do
 current=$(identity_snapshot || true)
 if [ -n "$current" ] && [ "$current" = "$previous" ]; then stable=$current; break; fi
 previous=$current
 sleep 0.1
 tries=$((tries+1))
done
[ -n "$stable" ] || { /bin/kill -TERM -- "-$process_group" 2>/dev/null || true; wait "$wrapper_pid" || true; exit 72; }
starttime=${stable%%:*}; remainder=${stable#*:}; stable_group=${remainder%%:*}; cmdline_sha=${remainder#*:}
[ "$stable_group" = "$process_group" ] || { /bin/kill -TERM -- "-$process_group" 2>/dev/null || true; wait "$wrapper_pid" || true; exit 72; }
setsid "$watchdog" "$solver_pid" "$process_group" "$starttime" "$cmdline_sha" 1789282740 "$run_dir/watchdog.log" > "$run_dir/watchdog.stdout" 2>&1 &
watchdog_pid=$!
expected_start="started leader_pid=$solver_pid process_group=$process_group starttime=$starttime cmdline_sha256=$cmdline_sha deadline_epoch=1789282740"
watchdog_live() { kill -0 "$watchdog_pid" 2>/dev/null && [ -r "/proc/$watchdog_pid/status" ] && awk '$1 == "State:" && $2 == "Z" { exit 1 }' "/proc/$watchdog_pid/status"; }
tries=0
while [ "$tries" -lt 100 ]; do grep -F "$expected_start" "$run_dir/watchdog.log" >/dev/null 2>&1 && watchdog_live && break; watchdog_live || { /bin/kill -TERM -- "-$process_group" 2>/dev/null || true; wait "$wrapper_pid" || true; exit 73; }; sleep 0.1; tries=$((tries+1)); done
grep -F "$expected_start" "$run_dir/watchdog.log" >/dev/null 2>&1 && watchdog_live || { /bin/kill -TERM -- "-$process_group" 2>/dev/null || true; wait "$wrapper_pid" || true; kill "$watchdog_pid" 2>/dev/null || true; exit 73; }
{
 echo time_pid="$wrapper_pid"; echo timeout_pid="$timeout_pid"; echo solver_pid="$solver_pid"; echo process_group="$process_group"; echo solver_starttime="$starttime"; echo solver_cmdline_sha256="$cmdline_sha"; echo expected_argv_sha256="$expected_cmdline_sha"; echo identity_stable_polls=2; echo watchdog_pid="$watchdog_pid"; echo watchdog_handshake=confirmed;
} >> "$run_dir/admission.txt"
printf 'handoff_at=%s\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" >> "$run_dir/admission.txt"
printf 'launch_handoff=confirmed run_dir=%s solver_pid=%s process_group=%s watchdog_pid=%s deadline_epoch=%s\n' "$run_dir" "$solver_pid" "$process_group" "$watchdog_pid" 1789282740
exit 0
