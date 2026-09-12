#!/bin/sh
set -eu

ROOT=$(git rev-parse --show-toplevel)
PLAN="$ROOT/evidence/p10/reference-spectrum-n384-planning-f6346da"
REFERENCE="$ROOT/work/p10-reference-spectrum-n384"
COMPARATOR="$ROOT/work/p10-force-snapshot-tool-n384"
OUTPUT="$ROOT/work/p10-reference-output-n384"
TARGET="$ROOT/work/p10-reference-n384-target"
BASE=f6346da8085365fad84874db08aa1213c75df331

cd "$ROOT"
test ! -e "$REFERENCE"
test ! -e "$COMPARATOR"
test ! -e "$OUTPUT"
test ! -e "$TARGET"
git diff --quiet "$BASE" -- Cargo.lock Cargo.toml crates/nsbu-benchmarks crates/nsbu-solver
sha256sum -c "$PLAN/source/input-sha256.txt"
mkdir -p "$REFERENCE/src" "$COMPARATOR/src" "$OUTPUT"
cp evidence/p10/reference-spectrum-high-band-9c90c0a/source/reference-Cargo.toml "$REFERENCE/Cargo.toml"
cp evidence/p10/reference-spectrum-high-band-9c90c0a/source/reference-Cargo.lock "$REFERENCE/Cargo.lock"
cp evidence/p10/reference-spectrum-high-band-9c90c0a/source/reference-spectrum-hi.rs "$REFERENCE/src/main.rs"
cp "$PLAN/source/high_bands.rs" "$REFERENCE/src/high_bands.rs"
cp evidence/p10/reference-spectrum-high-band-9c90c0a/source/comparator-Cargo.toml "$COMPARATOR/Cargo.toml"
cp evidence/p10/reference-spectrum-high-band-9c90c0a/source/comparator-Cargo.lock "$COMPARATOR/Cargo.lock"
cp evidence/p10/reference-spectrum-high-band-9c90c0a/source/comparator-hi.rs "$COMPARATOR/src/main.rs"
patch -d "$REFERENCE" -p0 < "$PLAN/source/reference-cargo.patch"
patch -d "$REFERENCE" -p0 < "$PLAN/source/reference-lock.patch"
patch -d "$REFERENCE" -p0 < "$PLAN/source/reference.patch"
patch -d "$COMPARATOR" -p0 < "$PLAN/source/comparator-cargo.patch"
patch -d "$COMPARATOR" -p0 < "$PLAN/source/comparator-lock.patch"
patch -d "$COMPARATOR" -p0 < "$PLAN/source/comparator.patch"
sha256sum -c "$PLAN/source/generated-harness-sha256.txt"
CARGO_TARGET_DIR="$TARGET" cargo build --release --manifest-path "$REFERENCE/Cargo.toml" --locked
FORENSICS_SOURCE="$BASE" CARGO_TARGET_DIR="$TARGET" cargo build --release --manifest-path "$COMPARATOR/Cargo.toml" --locked
ulimit -v 20971520
"$TARGET/release/p10-reference-spectrum-n384" 384 --preflight
"$TARGET/release/p10-reference-spectrum-n384" 768 --preflight
