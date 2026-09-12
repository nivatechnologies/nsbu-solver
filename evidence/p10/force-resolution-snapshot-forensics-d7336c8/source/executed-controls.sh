#!/bin/sh
set -u
T=work/p10-force-snapshot-tool/target/release/p10-force-snapshot-tool
O=work/p10-force-snapshot-forensics
S=$O/snapshots/m24-w12-clock2048.coeff.bin
H=ae1245a87c7e7a81ada6bb37fc0be8c620b5b358bb6a802e2e35633375ffb1ea
run_status() { name=$1; shift; /usr/bin/time -v "$T" "$@" >"$O/$name.stdout" 2>"$O/$name.stderr"; printf '%s\n' $? >"$O/$name.status"; }
run_status control-self compare "$S" "$S" 2048 12 12 24 24 12 12 "$H" "$H"
run_status control-wrong-hash compare "$S" "$S" 2048 12 12 24 24 12 12 0000000000000000000000000000000000000000000000000000000000000000 "$H"
run_status control-truncated compare "$O/snapshots/truncated.bin" "$S" 2048 12 12 24 24 12 12 a7972d79e72fce0f96d50104e488be7dd2dbea5b8779295364eaaf21919569bc "$H"
run_status control-nonfinite compare "$O/snapshots/nonfinite.bin" "$S" 2048 12 12 24 24 12 12 3769812c6c8e93abae449e26d10c1ed6b8f3a558decc6594f8522f57c51a1cf9 "$H"
A=/mnt/niva-array/nsbu-solver/work/p10-resolution-pilot-corrected/work/p10-resolution-pilot-corrected-487c330-full/checkpoints/accepted-event-3-clock2048-branch0.bin
run_status control-wrong-clock extract "$A" /tmp/must-not-exist.coeff.bin 4096 24 12
