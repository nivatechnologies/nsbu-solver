#!/usr/bin/env bash
# Admission-refusal tests for the N512 clock-1536 balance manifest.
# Every check is executed through the REAL reviewed Rust read_manifest path via
# the offline-captured-observer `preflight` CLI (arithmetic only: the CLI never
# opens the snapshot file in this mode and allocates no state).
set -u
DIR=$(cd "$(dirname "$0")" && pwd)
EVID=$(dirname "$DIR")
TARGET_DIR=${CARGO_TARGET_DIR:-"$EVID/../offline-captured-observer/harness/target"}
OBS="$TARGET_DIR/release/p10-offline-captured-observer"
GOOD="$EVID/manifests/balance-clock1536-unexecuted-template.json"
BUILDER="$EVID/builder/build_balance_manifest.py"
RECORD="$EVID/../offline-captured-observer/harness/clock1536-record.json"
PLAN="$EVID/../n512-m512-endpoint-prep-20260913/proposed-launch/v3-launch-plan.json"
TRAJ="$EVID/../n512-m512-endpoint-prep-20260913/frozen-plan.json"
OUT="$DIR/artifacts"
SAMPLE=1024
WORKERS=32
BACKEND=rustfft-6.4.1-avx-avx2-fma
CAP=274877906944
rm -rf "$OUT"; mkdir -p "$OUT"
pass=0; fail=0

mutate() { # name, python-mutation-body (mutates `m`)
  python3 - "$GOOD" "$OUT/$1.json" "$PLAN" "$OUT" <<PY
import json, os, sys
m = json.load(open(sys.argv[1]))
m["plan"] = os.path.relpath(sys.argv[3], sys.argv[4])
$2
json.dump(m, open(sys.argv[2], "w"), indent=2)
PY
}

expect_refusal() { # name, manifest, expected-substring
  local name=$1 manifest=$2 expected=$3
  "$OBS" preflight "$manifest" "$SAMPLE" "$WORKERS" "$CAP" "$BACKEND" \
      >"$OUT/$name.stdout" 2>"$OUT/$name.stderr"
  local status=$?
  if [ $status -ne 0 ] && grep -qF "$expected" "$OUT/$name.stderr"; then
    echo "PASS refuse $name"; pass=$((pass+1))
  else
    echo "FAIL refuse $name (status=$status, expected '$expected')"; fail=$((fail+1))
    cat "$OUT/$name.stderr"
  fi
}

expect_accept() { # name, manifest
  local name=$1 manifest=$2
  "$OBS" preflight "$manifest" "$SAMPLE" "$WORKERS" "$CAP" "$BACKEND" \
      >"$OUT/$name.stdout" 2>"$OUT/$name.stderr"
  local status=$?
  if [ $status -eq 0 ] && grep -q '"scope": "balance-diagnostic-only"' "$OUT/$name.stdout" \
     && grep -q '"qualification": false' "$OUT/$name.stdout"; then
    echo "PASS accept $name"; pass=$((pass+1))
  else
    echo "FAIL accept $name (status=$status)"; fail=$((fail+1)); cat "$OUT/$name.stderr"
  fi
}

builder_expect_refusal() { # name, expected-substring, extra-args...
  local name=$1 expected=$2; shift 2
  python3 "$BUILDER" "$@" >"$OUT/$name.stdout" 2>"$OUT/$name.stderr"
  local status=$?
  if [ $status -ne 0 ] && grep -qE "$expected" "$OUT/$name.stderr" "$OUT/$name.stdout"; then
    echo "PASS builder-refuse $name"; pass=$((pass+1))
  else
    echo "FAIL builder-refuse $name (status=$status, expected '$expected')"; fail=$((fail+1))
    cat "$OUT/$name.stderr" "$OUT/$name.stdout"
  fi
}

# --- the unexecuted template itself: admitted, arithmetic-only, unqualified ---
expect_accept good "$GOOD"

# --- wrong case ---
mutate wrong-case 'm["evolution"]["case_sha256"] = "f"*64'
expect_refusal wrong-case "$OUT/wrong-case.json" "unsupported case profile"
mutate wrong-case-char 'm["evolution"]["case_sha256"] = m["evolution"]["case_sha256"][:-1] + ("0" if m["evolution"]["case_sha256"][-1] != "0" else "1")'
expect_refusal wrong-case-char "$OUT/wrong-case-char.json" "unsupported case profile"

# --- wrong clock ---
mutate wrong-clock-elapsed 'm["elapsed"] = 1600'
expect_refusal wrong-clock-elapsed "$OUT/wrong-clock-elapsed.json" "invalid evolution semantics"
mutate wrong-clock-target 'm["elapsed"]=4096; m["target"]=4096; m["evolution"]["comparison_endpoint"]=4096; m["evolution"]["clock_target"]=4096; m["evolution"]["schedule"][0]["until_exclusive"]=2048; m["evolution"]["schedule"].append({"from_inclusive":2048,"until_exclusive":4096,"step_ticks":128})'
expect_refusal wrong-clock-target "$OUT/wrong-clock-target.json" "zero remaining ticks"

# --- wrong step schedule (prefix convention broken) ---
mutate wrong-schedule-full 'm["evolution"]["schedule"]=[{"from_inclusive":0,"until_exclusive":2048,"step_ticks":64},{"from_inclusive":2048,"until_exclusive":4096,"step_ticks":128}]'
expect_refusal wrong-schedule-full "$OUT/wrong-schedule-full.json" "does not reach target"
mutate wrong-schedule-gap 'm["evolution"]["schedule"][0]["from_inclusive"] = 64'
expect_refusal wrong-schedule-gap "$OUT/wrong-schedule-gap.json" "invalid piecewise schedule"
mutate wrong-schedule-step 'm["evolution"]["schedule"][0]["step_ticks"] = 320'
expect_refusal wrong-schedule-step "$OUT/wrong-schedule-step.json" "invalid piecewise schedule"

# --- wrong hash bindings ---
mutate wrong-plan-hash 'm["plan_sha256"] = m["plan_sha256"][:-1] + ("0" if m["plan_sha256"][-1] != "0" else "1")'
expect_refusal wrong-plan-hash "$OUT/wrong-plan-hash.json" "frozen plan SHA-256 mismatch"
mutate cross-plan 'm["plan"] = os.path.relpath("'"$TRAJ"'", sys.argv[4])'
expect_refusal cross-plan "$OUT/cross-plan.json" "frozen plan SHA-256 mismatch"
mutate wrong-coefficient-hash 'm["coefficient_sha256"] = "zz"*32'
expect_refusal wrong-coefficient-hash "$OUT/wrong-coefficient-hash.json" "invalid comparison manifest binding"
mutate wrong-file-hash 'm["file_sha256"] = m["file_sha256"][:63]'
expect_refusal wrong-file-hash "$OUT/wrong-file-hash.json" "invalid comparison manifest binding"

# --- wrong profile/kind/method/force/backend envelopes ---
mutate wrong-kind-method-decode 'm["comparison_kind"] = "METHOD_DIAGNOSTIC"'
expect_refusal wrong-kind-method-decode "$OUT/wrong-kind-method-decode.json" "invalid evolution semantics"
mutate wrong-kind-method 'm["comparison_kind"] = "METHOD_DIAGNOSTIC"; m["dimensions"]=[384,384,384]; m["evolution"]["integration_force_dimensions"]=[384,384,384]'
expect_refusal wrong-kind-method "$OUT/wrong-kind-method.json" "method-diagnostic"
mutate wrong-kind-matched 'm["comparison_kind"] = "MATCHED_SPATIAL"'
expect_refusal wrong-kind-matched "$OUT/wrong-kind-matched.json" "invalid evolution semantics"
mutate wrong-method 'm["evolution"]["method"] = "hochbruck-ostermann"'
expect_refusal wrong-method "$OUT/wrong-method.json" "invalid evolution semantics"
mutate wrong-force-dims 'm["evolution"]["integration_force_dimensions"] = [1024,1024,1024]'
expect_refusal wrong-force-dims "$OUT/wrong-force-dims.json" "invalid evolution semantics"
mutate wrong-backend-envelope 'm["backend"] = ""'
expect_refusal wrong-backend-envelope "$OUT/wrong-backend-envelope.json" "invalid comparison manifest binding"
"$OBS" preflight "$GOOD" "$SAMPLE" "$WORKERS" "$CAP" fftw >"$OUT/wrong-backend-cli.stdout" 2>"$OUT/wrong-backend-cli.stderr"
if [ $? -ne 0 ] && grep -qF "unsupported FFT backend" "$OUT/wrong-backend-cli.stderr"; then
  echo "PASS refuse wrong-backend-cli"; pass=$((pass+1))
else echo "FAIL refuse wrong-backend-cli"; fail=$((fail+1)); fi

# --- resource ledger: cap-exact admits, cap-minus-one reports fits=false ---
TOTAL=$(python3 -c "import json,sys; print(json.load(open('$OUT/good.stdout'))['ledger']['total_bytes'])")
"$OBS" preflight "$GOOD" "$SAMPLE" "$WORKERS" "$TOTAL" "$BACKEND" >"$OUT/cap-exact.stdout" 2>"$OUT/cap-exact.stderr"
if [ $? -eq 0 ] && grep -q '"fits": true' "$OUT/cap-exact.stdout"; then
  echo "PASS cap-exact fits"; pass=$((pass+1))
else echo "FAIL cap-exact fits"; fail=$((fail+1)); fi
"$OBS" preflight "$GOOD" "$SAMPLE" "$WORKERS" "$((TOTAL-1))" "$BACKEND" >"$OUT/cap-under.stdout" 2>"$OUT/cap-under.stderr"
if [ $? -eq 0 ] && grep -q '"fits": false' "$OUT/cap-under.stdout"; then
  echo "PASS cap-minus-one fits=false"; pass=$((pass+1))
else echo "FAIL cap-minus-one fits=false"; fail=$((fail+1)); fi

# --- builder refusals (wrong clock/step records, unknown or invalid hashes) ---
python3 - "$RECORD" "$OUT/record-clock.json" <<'PY'
import json, sys
r = json.load(open(sys.argv[1])); r["clock"] = 1600
json.dump(r, open(sys.argv[2], "w"))
PY
builder_expect_refusal builder-clock "clock record is not the frozen|record field" --record "$OUT/record-clock.json" --plan "$PLAN" --output "$OUT/builder-clock.json" --unexecuted-placeholder
python3 - "$RECORD" "$OUT/record-steps.json" <<'PY'
import json, sys
r = json.load(open(sys.argv[1])); r["accepted_steps"] = 23
json.dump(r, open(sys.argv[2], "w"))
PY
builder_expect_refusal builder-steps "clock record is not the frozen" --record "$OUT/record-steps.json" --plan "$PLAN" --output "$OUT/builder-steps.json" --unexecuted-placeholder
python3 - "$RECORD" "$OUT/record-hash.json" <<'PY'
import json, sys
r = json.load(open(sys.argv[1])); r["state_sha256"] = "a"*64
json.dump(r, open(sys.argv[2], "w"))
PY
builder_expect_refusal builder-hash "clock record is not the frozen|state_sha256" --record "$OUT/record-hash.json" --plan "$PLAN" --output "$OUT/builder-hash.json" --unexecuted-placeholder
builder_expect_refusal builder-plan "SHA-256 mismatch|launch plan" --record "$RECORD" --plan "$TRAJ" --output "$OUT/builder-plan.json" --unexecuted-placeholder
builder_expect_refusal builder-nohash "externally reviewed" --record "$RECORD" --plan "$PLAN" --output "$OUT/builder-nohash.json"
builder_expect_refusal builder-badhex "hex" --record "$RECORD" --plan "$PLAN" --output "$OUT/builder-badhex.json" --file-sha256 "nope"
builder_expect_refusal builder-zero "only allowed" --record "$RECORD" --plan "$PLAN" --output "$OUT/builder-zero.json" --file-sha256 "$(printf '0%.0s' {1..64})"

# --- builder accepts the reviewed inputs and reproduces the template byte-exactly ---
python3 "$BUILDER" --record "$RECORD" --plan "$PLAN" --output "$OUT/builder-good.json" --unexecuted-placeholder >"$OUT/builder-good.stdout" 2>&1
if [ $? -eq 0 ] && python3 -c "
import json
a = json.load(open('$OUT/builder-good.json')); b = json.load(open('$GOOD'))
a['plan'] = b['plan'] = 0
raise SystemExit(0 if a == b else 1)
"; then
  echo "PASS builder-reproducible"; pass=$((pass+1))
else echo "FAIL builder-reproducible"; fail=$((fail+1)); fi

# --- create-only atomic publication: replay, race, and alias refusals ---
python3 "$BUILDER" --record "$RECORD" --plan "$PLAN" --output "$OUT/replay.json" --unexecuted-placeholder >"$OUT/builder-first.stdout" 2>&1
if [ $? -eq 0 ]; then echo "PASS builder-first-publish"; pass=$((pass+1)); else echo "FAIL builder-first-publish"; fail=$((fail+1)); fi
builder_expect_refusal builder-replay "already exists" --record "$RECORD" --plan "$PLAN" --output "$OUT/replay.json" --unexecuted-placeholder
python3 "$BUILDER" --record "$RECORD" --plan "$PLAN" --output "$OUT/race.json" --unexecuted-placeholder >"$OUT/race-a.stdout" 2>&1 & race_a=$!
python3 "$BUILDER" --record "$RECORD" --plan "$PLAN" --output "$OUT/race.json" --unexecuted-placeholder >"$OUT/race-b.stdout" 2>&1 & race_b=$!
wait $race_a; status_a=$?; wait $race_b; status_b=$?
successes=0
[ $status_a -eq 0 ] && successes=$((successes+1))
[ $status_b -eq 0 ] && successes=$((successes+1))
if [ $successes -eq 1 ] && [ -f "$OUT/race.json" ]; then
  echo "PASS builder-race exactly-one-winner"; pass=$((pass+1))
else echo "FAIL builder-race (successes=$successes)"; fail=$((fail+1)); cat "$OUT/race-a.stdout" "$OUT/race-b.stdout"; fi
ln -sf "$PLAN" "$OUT/alias-symlink.json"
builder_expect_refusal builder-alias-symlink "aliases a builder input" --record "$RECORD" --plan "$PLAN" --output "$OUT/alias-symlink.json" --unexecuted-placeholder
ln "$RECORD" "$OUT/alias-hardlink.json"
builder_expect_refusal builder-alias-hardlink "already exists" --record "$RECORD" --plan "$PLAN" --output "$OUT/alias-hardlink.json" --unexecuted-placeholder
builder_expect_refusal builder-alias-input "aliases a builder input" --record "$RECORD" --plan "$PLAN" --output "$RECORD" --unexecuted-placeholder
if [ "$(sha256sum "$RECORD" | cut -d' ' -f1)" = "4fbb8c213a41a5e88eca4599953e3fa4cd5d4a2af8975abe6062b51a066d6376" ]; then
  echo "PASS frozen-record-untouched"; pass=$((pass+1))
else echo "FAIL frozen-record-untouched"; fail=$((fail+1)); fi
if ! ls -A "$OUT" | grep -q '\.tmp-'; then
  echo "PASS no-publication-debris"; pass=$((pass+1))
else echo "FAIL no-publication-debris"; ls -A "$OUT" | grep '\.tmp-'; fail=$((fail+1)); fi

# --- focused Python unit tests for the binding/publication gates ---
if python3 "$DIR/builder_unit.py" >"$OUT/builder-unit.stdout" 2>&1; then
  echo "PASS builder-unit-suite"; pass=$((pass+1))
else echo "FAIL builder-unit-suite"; fail=$((fail+1)); cat "$OUT/builder-unit.stdout"; fi

echo "== tests: $pass passed, $fail failed"
echo "$fail" > "$DIR/refusals.status"
[ $fail -eq 0 ]
