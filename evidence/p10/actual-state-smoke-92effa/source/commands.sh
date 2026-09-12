#!/bin/sh
set -eu
SOURCE=/mnt/niva-array/nsbu-solver/work/p10-state-smoke-source-92effa
HARNESS=/mnt/niva-array/nsbu-solver/work/p10-state-smoke-harness
TARGET=/mnt/niva-array/nsbu-solver/work/p10-state-smoke-target
STATE=/mnt/niva-array/nsbu-solver/work/p10-fft-batch-20260912/evidence/p10/avx-scheduled-endpoint/run-92effa6/node-0512/state.bin
test "$(git -C "$SOURCE" rev-parse HEAD)" = 92effa6068d20d69e815c6a83f1e82490ce37fe7
sha256sum -c evidence/p10/actual-state-smoke-92effa/source/input-sha256.txt
test ! -e "$HARNESS"
mkdir -p "$HARNESS/src"
cp evidence/p10/actual-state-smoke-92effa/source/Cargo.toml "$HARNESS/Cargo.toml"
cp evidence/p10/actual-state-smoke-92effa/source/Cargo.lock "$HARNESS/Cargo.lock"
cp evidence/p10/actual-state-smoke-92effa/source/analyzer.rs "$HARNESS/src/main.rs"
(cd evidence/p10/actual-state-smoke-92effa/source && sha256sum -c harness-sha256.txt)
CARGO_TARGET_DIR="$TARGET" cargo build --release --offline --manifest-path "$HARNESS/Cargo.toml"
"$TARGET/release/p10-actual-state-smoke" "$STATE"
