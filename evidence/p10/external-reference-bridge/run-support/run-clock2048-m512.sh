#!/usr/bin/env bash
set -euo pipefail

root=$(git rev-parse --show-toplevel)
base="$root/evidence/p10/external-reference-bridge"
binary="$base/harness/target/release/p10-external-reference-bridge"
bridge="$base/inputs/clock2048-m512/bridge.json"
run_dir=${RUN_DIR:?set RUN_DIR to a new output directory}
candidate="$run_dir/diagnostic.json.candidate"
final="$run_dir/diagnostic.json"
stderr="$run_dir/stderr.jsonl"
time_file="$run_dir/time.txt"
mkdir -p "$run_dir"
test ! -e "$candidate" && test ! -e "$final" && test ! -e "$stderr" && test ! -e "$time_file"
printf '%s  %s\n' 4d18638cc040d86e48eb330daff8ee46db985fef23a9247b7ad2df990f9f3a1d "$binary" | sha256sum -c -
printf '%s  %s\n' a975f09bacd6ecdd1c47d6918db822a1560ff8b670559e0300b270b16d85433a "$bridge" | sha256sum -c -
"$binary" "$bridge" 20900924104 > "$run_dir/preflight.json"
set +e
/usr/bin/time -v -o "$time_file" timeout --signal=TERM --kill-after=30s 1800 \
  "$binary" "$bridge" 20900924104 --execute > "$candidate" 2> "$stderr"
status=$?
set -e
if (( status != 0 )); then
  printf 'bounded execution incomplete (exit %d); candidate, stderr, and time retained\n' "$status" >&2
  exit "$status"
fi
python3 "$base/run-support/validate_candidate.py" "$candidate" "$bridge" "$binary"
mv "$candidate" "$final"
printf 'promoted %s\n' "$final"
