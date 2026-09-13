#!/usr/bin/env bash
set -euo pipefail

root=$(git rev-parse --show-toplevel)
base="$root/evidence/p10/external-reference-bridge"
binary="$base/harness/target/release/p10-external-reference-bridge"
bridge="$base/inputs/clock1024-m512/bridge.json"
run_dir=${RUN_DIR:?set RUN_DIR to a new output directory}
candidate="$run_dir/diagnostic.json.candidate"
final="$run_dir/diagnostic.json"
stderr="$run_dir/stderr.jsonl"
time_file="$run_dir/time.txt"
mkdir -p "$run_dir"
test ! -e "$candidate" && test ! -e "$final" && test ! -e "$stderr" && test ! -e "$time_file"
printf '%s  %s\n' 87656e3654a5a9c1a3861a785f9f856e5d3ba3dadeda48a0215ea73964eb6ebc "$binary" | sha256sum -c -
printf '%s  %s\n' 9fde34d036493db73864bedb7c214e18e71047b017698a8593e93c2274e6d3e5 "$bridge" | sha256sum -c -
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
