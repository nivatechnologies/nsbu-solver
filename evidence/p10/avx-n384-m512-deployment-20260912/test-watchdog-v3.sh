#!/bin/sh
set -eu
HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
TMP=$(mktemp -d /tmp/p10-watchdog-v3.XXXXXX)
trap 'jobs -p | xargs -r kill 2>/dev/null || true' EXIT HUP INT TERM
cat >"$TMP/transient.c" <<'C'
#include <string.h>
#include <unistd.h>
int main(int argc, char **argv) {
    (void)argc;
    size_t n = strlen(argv[0]);
    char original[4096];
    if (n >= sizeof original) return 2;
    memcpy(original, argv[0], n + 1);
    usleep(800000);
    memset(argv[0], 'x', n);
    usleep(800000);
    memcpy(argv[0], original, n);
    for (;;) sleep(1);
}
C
cc -O2 -Wall -Wextra -Werror -o "$TMP/transient-dummy" "$TMP/transient.c"
setsid "$TMP/transient-dummy" &
leader=$!
for _ in $(seq 1 50); do rest=$(cat "/proc/$leader/stat"); rest=${rest##*) }; set -- $rest; [ "$3" = "$leader" ] && break; sleep 0.02; done
pgid=$3; starttime=${20}; cmdsha=$(sha256sum "/proc/$leader/cmdline" | awk '{print $1}')
WATCHDOG_POLL_SECONDS=1 setsid "$HERE/pgid-watchdog-v3.sh" "$leader" "$pgid" "$starttime" "$cmdsha" "$(( $(date +%s) + 30 ))" "$TMP/transient.log" &
watchdog=$!
for _ in $(seq 1 60); do grep -q identity_sample_recovered "$TMP/transient.log" 2>/dev/null && break; sleep 0.1; done
grep -q 'identity_sample_failed component=cmdline_sha256 ' "$TMP/transient.log"
grep -q 'identity_sample_recovered prior_component=cmdline_sha256 ' "$TMP/transient.log"
/bin/kill -TERM -- "-$pgid"
wait "$leader" 2>/dev/null || true
wait "$watchdog"
! grep -q deadline_reached_sending_TERM "$TMP/transient.log"

setsid sh -c 'trap "exit 0" TERM; while :; do sleep 1; done' &
leader=$!
for _ in $(seq 1 50); do rest=$(cat "/proc/$leader/stat"); rest=${rest##*) }; set -- $rest; [ "$3" = "$leader" ] && break; sleep 0.02; done
pgid=$3; starttime=${20}; cmdsha=$(sha256sum "/proc/$leader/cmdline" | awk '{print $1}')
WATCHDOG_POLL_SECONDS=1 setsid "$HERE/pgid-watchdog-v3.sh" "$leader" "$pgid" "$starttime" "$cmdsha" "$(( $(date +%s) + 2 ))" "$TMP/deadline.log" &
watchdog=$!
wait "$watchdog"
wait "$leader" 2>/dev/null || true
grep -q deadline_reached_sending_TERM "$TMP/deadline.log"
echo 'transient_recovery=passed no_premature_TERM=passed explicit_deadline_TERM=passed'
