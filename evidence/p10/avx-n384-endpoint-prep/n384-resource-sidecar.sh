#!/bin/sh
set -eu

pid=$1
expected_starttime=$2
expected_cmdline_sha256=$3
deadline=$4
log=$5

while [ "$(date +%s)" -lt "$deadline" ] && [ -r "/proc/$pid/stat" ]; do
    stat=$(cat "/proc/$pid/stat") || break
    rest=${stat##*) }
    set -- $rest
    [ "$1" != Z ] || break
    [ "${20}" = "$expected_starttime" ] || break
    actual=$(sha256sum "/proc/$pid/cmdline" | awk '{print $1}') || break
    [ "$actual" = "$expected_cmdline_sha256" ] || break
    {
        printf 'utc=%s ' "$(date -u +%Y-%m-%dT%H:%M:%SZ)"
        awk '/^(VmRSS|VmHWM|VmSwap|Threads):/ {printf "%s=%s%s ",$1,$2,$3} END {print ""}' "/proc/$pid/status"
        awk '/MemAvailable:/ {print "MemAvailable_kib=" $2}' /proc/meminfo
        ps -L -o psr= -p "$pid" | sort -n | uniq -c | tr '\n' ' '; echo
        numastat -p "$pid" 2>&1
    } >> "$log"
    sleep 30
done
printf 'utc=%s monitor_exit\n' "$(date -u +%Y-%m-%dT%H:%M:%SZ)" >> "$log"
