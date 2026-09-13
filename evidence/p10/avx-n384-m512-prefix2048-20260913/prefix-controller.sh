#!/bin/sh
set -eu
[ "$#" -eq 1 ] || { echo "usage: $0 BUNDLE" >&2; exit 64; }
BUNDLE=$(CDPATH= cd -- "$1" && pwd)
PLAN=$BUNDLE/frozen-prefix-plan.json
PLAN_SHA256=e44ca2db9f667749e8b33b88f346f1880f0c1b4961e16e14cba2fe071e0b3a84
DEADLINE_EPOCH=1789282800
[ "$(sha256sum "$PLAN" | awk '{print $1}')" = "$PLAN_SHA256" ] || exit 65
RUN_ROOT=$BUNDLE/run
LOG_DIR=$RUN_ROOT/logs
OUTPUT=$RUN_ROOT/output
IDENTITY=$LOG_DIR/launch-identity.txt
[ -r "$IDENTITY" ] || exit 66
value() { sed -n "s/^$1=//p" "$IDENTITY"; }
TIME_PID=$(value time_pid)
SOLVER_PID=$(value solver_pid)
PROCESS_GROUP=$(value process_group)
STARTTIME=$(value starttime)
CMDLINE_SHA256=$(value cmdline_sha256)
WATCHDOG_PID=$(value watchdog_pid)
TIME_STARTTIME=
TIME_CMDLINE_SHA256=
SPAWN_PENDING=0
LAUNCH_STARTED=1
LAUNCH_HANDOFF=0
CLEANUP_GRACE_SECONDS=60
. "$BUNDLE/launcher-owned-cleanup.sh"
install_owned_cleanup_traps
record() { printf '%s %s\n' "$(date -u +%FT%TZ)" "$1" >>"$LOG_DIR/prefix-policy.log"; }
stop_at_checkpoint() {
    checkpoint=$1
    budget=$2
    remaining=$3
    record "stopping_after_durable_prefix clock=$checkpoint segment_budget=$budget remaining=$remaining claim=feasibility_incomplete_no_endpoint_no_window"
    cleanup_owned_group
    LAUNCH_HANDOFF=1
    exit 0
}
for checkpoint in 2048 3072; do
    case "$checkpoint" in 2048) attempt=32 ;; 3072) attempt=40 ;; esac
    while ! grep -q "^attempt=$attempt clock=$checkpoint " "$LOG_DIR/stdout"; do
        owned_member_matches "$SOLVER_PID" "$PROCESS_GROUP" "$STARTTIME" "$CMDLINE_SHA256" || exit 67
        kill -0 "$WATCHDOG_PID" 2>/dev/null || exit 68
        sleep 10
    done
    step=$(printf '%s/step-%03d-clock-%04d' "$OUTPUT" "$attempt" "$checkpoint")
    [ -s "$step/attempt.json" ] && [ -s "$step/record.json" ] || exit 69
    [ "$(stat -c %s "$step/state.bin")" -eq 1366033529 ] || exit 70
    first=$((attempt - 7))
    set -- $(awk -v first="$first" -v last="$attempt" '
        /^attempt=/ {
            split($1,a,"="); n=a[2]+0
            if (n >= first && n <= last) for (i=1;i<=NF;i++) if ($i ~ /^integration_seconds=/) {split($i,v,"="); if (v[2]+0>imax) imax=v[2]+0}
            if (n <= last) for (i=1;i<=NF;i++) if ($i ~ /^observer_seconds=Some\(/) {v=$i; sub(/^observer_seconds=Some\(/,"",v); sub(/\).*/,"",v); if (v+0>omax) omax=v+0}
        }
        END {printf "%.12f %.12f\n", imax, omax}
    ' "$LOG_DIR/stdout")
    imax=$1
    omax=$2
    awk -v i="$imax" -v o="$omax" 'BEGIN{exit !(i>0 && o>0)}' || exit 71
    segment_budget=$(awk -v i="$imax" -v o="$omax" 'BEGIN{x=1.15*(8*i+2*o+300); print int(x)+(x>int(x))}')
    now=$(date +%s)
    remaining=$((DEADLINE_EPOCH - now))
    record "durable_checkpoint clock=$checkpoint imax_recent8=$imax omax_observed=$omax segment_budget=$segment_budget remaining=$remaining"
    [ "$segment_budget" -le "$remaining" ] || stop_at_checkpoint "$checkpoint" "$segment_budget" "$remaining"
done
record "continuation_to_4096_admitted claim_remains_feasibility_until_original_binary_terminal"
LAUNCH_HANDOFF=1
