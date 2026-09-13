#!/bin/bash
set -u

source_tree=/mnt/niva-array/nsbu-solver/work/p10-v2-layout-repair-split-20260913
target_dir=/tmp/nsbu-terra-avx-tail-final-workspace-20260913
receipt=/tmp/nsbu-terra-avx-tail-final-workspace-continuation-review-20260913
deadline=1789328874

run_phase() {
    name=$1
    shift
    remaining=$((deadline - $(date -u +%s)))
    if [ "$remaining" -le 60 ]; then
        printf 'deadline_refusal remaining_seconds=%s\n' "$remaining" > "$receipt/$name.stderr"
        printf '125\n' > "$receipt/$name.status"
        return 125
    fi
    printf 'start_utc=%s\nremaining_seconds=%s\n' "$(date -u +%FT%TZ)" "$remaining" > "$receipt/$name.timing"
    set +e
    timeout -k 60s "${remaining}s" env CARGO_TARGET_DIR="$target_dir" CARGO_BUILD_JOBS=2 \
        "$@" > "$receipt/$name.stdout" 2> "$receipt/$name.stderr"
    status=$?
    set -e
    printf 'end_utc=%s\nstatus=%s\n' "$(date -u +%FT%TZ)" "$status" >> "$receipt/$name.timing"
    printf '%s\n' "$status" > "$receipt/$name.status"
    return "$status"
}

cd "$source_tree"
[ "$(git rev-parse HEAD)" = 0843b8b18e6a096a0208e3d896e391c7b1b2f5e0 ] || exit 3
[ -z "$(git status --porcelain)" ] || exit 4
[ "$(ps -eo pgid= | awk '$1==1956764{n++} END{print n+0}')" = 0 ] || exit 5
[ ! -e /proc/2127386 ] || exit 6

mapfile -t tests < "$receipt/targets.txt"
args=()
for test_name in "${tests[@]}"; do
    args+=(--test "$test_name")
done

overall=0
run_phase benchmarks cargo test -p nsbu-benchmarks --locked --no-fail-fast \
    "${args[@]}" -- --test-threads=1 || overall=1
run_phase cli cargo test -p nsbu-cli --locked --no-fail-fast --bins --tests \
    -- --test-threads=1 || overall=1
run_phase doctests cargo test --workspace --locked --doc --no-fail-fast \
    -- --test-threads=1 || overall=1
printf '%s\n' "$overall" > "$receipt/overall.status"
sha256sum "$receipt"/*.stdout "$receipt"/*.stderr "$receipt"/*.status \
    "$receipt"/*.timing > "$receipt/RESULT_SHA256SUMS"
exit "$overall"
