#!/bin/sh
set -eu
# Exact executed commands. Absolute paths bind the recorded execution.
TOOL=/mnt/niva-array/nsbu-solver/work/p10-force-snapshot-forensics/work/p10-force-snapshot-tool/target/release/p10-force-snapshot-tool
ROOT=/mnt/niva-array/nsbu-solver/work/p10-cli-m96-feasibility/work/m96-relaunch-midpoint-2048
OUT=/mnt/niva-array/nsbu-solver/work/p10-force-snapshot-forensics/work/p10-force-snapshot-tool/fresh-midpoint
/usr/bin/time -v "$TOOL" compare "$ROOT/n48/forensics/accepted-n48-m96-cm16-clock2048.coeff.bin" "$ROOT/n64/forensics/accepted-n64-m96-cm16-clock2048.coeff.bin" 2048 48 64 96 96 32 32 383c02753fa73b13bef107d4635e5083c6b8b7bf9b02e724988bd0c009e6adb3 22db0441ba7561aa350804d3d25e08203119d228cebd3160a5356003ca5fe2d4 > "$OUT/n48-n64.stdout" 2> "$OUT/n48-n64.time"
/usr/bin/time -v "$TOOL" compare "$OUT/zero-n64.coeff.bin" "$ROOT/n64/forensics/accepted-n64-m96-cm16-clock2048.coeff.bin" 2048 64 64 96 96 32 32 2f67df2fdeae7d555b20ad7c4cc7409ce38860f3e4846e57183c0896a9c6c30e 22db0441ba7561aa350804d3d25e08203119d228cebd3160a5356003ca5fe2d4 > "$OUT/n64-norm.stdout" 2> "$OUT/n64-norm.time"
