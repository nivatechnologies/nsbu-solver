#!/usr/bin/env bash
set -euo pipefail

root=$(git rev-parse --show-toplevel)
base="$root/evidence/p10/external-reference-bridge"
binary="$base/harness/target/release/p10-external-reference-bridge"
bridge="$base/inputs/clock3072-m512/bridge.json"
run_dir=${RUN_DIR:?set RUN_DIR to a new output directory}
candidate="$run_dir/diagnostic.json.candidate"
final="$run_dir/diagnostic.json"
stderr="$run_dir/stderr.jsonl"
time_file="$run_dir/time.txt"
mkdir -p "$run_dir"
test ! -e "$candidate" && test ! -e "$final" && test ! -e "$stderr" && test ! -e "$time_file"
printf '%s  %s\n' 782444a29870e1a678fa8393149bd9ba0c28e4cc08dff3d635a7a0bb98c3cb52 "$binary" | sha256sum -c -
printf '%s  %s\n' c6f1fdb94617dadc11516cf3c1bdb30c49806b6bd3ce9c4e2b5adc27db8428b0 "$bridge" | sha256sum -c -
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
