#!/bin/sh
set -eu
stage=/tmp/nsbu-p10-n512-m512-v3-20260913T2244Z
archive_parent=/tmp/nsbu-p10-n512-m512-v3-archive-20260913T2244Z
status_dir=$stage/unattended
launch_epoch=$(date +%s)
deadline_epoch=$((launch_epoch + 72000))
launch_utc=$(date -u -d "@$launch_epoch" +%FT%TZ)
deadline_utc=$(date -u -d "@$deadline_epoch" +%FT%TZ)
stat=$(cat "/proc/$$/stat")
rest=${stat##*) }
set -- $rest
pgid=$3
starttime=${20}
cmdline_sha256=$(sha256sum "/proc/$$/cmdline" | awk '{print $1}')
tmp=$status_dir/outer-start.json.tmp
cat >"$tmp" <<JSON
{"schema":"p10-n512-m512-unattended-start-v1","host":"$(hostname)","outer_pid":$$,"outer_pgid":$pgid,"outer_starttime":$starttime,"outer_cmdline_sha256":"$cmdline_sha256","launch_epoch":$launch_epoch,"launch_utc":"$launch_utc","deadline_epoch":$deadline_epoch,"deadline_utc":"$deadline_utc","stage":"$stage","archive_parent":"$archive_parent","outer_status":"$status_dir/outer-exit.status","outer_log":"$status_dir/outer.log","launch_authorized":true}
JSON
mv "$tmp" "$status_dir/outer-start.json"
set +e
NSBU_LAUNCH_N512_M512_ENDPOINT=1 \
NSBU_N512_M512_EXPECTED_BUNDLE="$stage" \
NSBU_N512_M512_DEADLINE_EPOCH="$deadline_epoch" \
NSBU_N512_M512_ARCHIVE_PARENT="$archive_parent" \
"$stage/launch-template.sh"
status=$?
set -e
printf '%s\n' "$status" >"$status_dir/outer-exit.status.tmp"
mv "$status_dir/outer-exit.status.tmp" "$status_dir/outer-exit.status"
printf '{"exit_status":%s,"finished_utc":"%s"}\n' "$status" "$(date -u +%FT%TZ)" >"$status_dir/outer-finish.json.tmp"
mv "$status_dir/outer-finish.json.tmp" "$status_dir/outer-finish.json"
exit "$status"
