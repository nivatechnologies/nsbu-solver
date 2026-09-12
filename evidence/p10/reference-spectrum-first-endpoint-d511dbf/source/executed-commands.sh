#!/bin/sh
# Actual working directory: repository root at d511dbfb8f5ae7681cdeaef6131ae7d198315bc7.
# The archived files reproduce the ignored work directory as follows.
mkdir -p work/p10-reference-spectrum/src
cp evidence/p10/reference-spectrum-first-endpoint-d511dbf/source/Cargo.toml work/p10-reference-spectrum/Cargo.toml
cp evidence/p10/reference-spectrum-first-endpoint-d511dbf/source/Cargo.lock work/p10-reference-spectrum/Cargo.lock
cp evidence/p10/reference-spectrum-first-endpoint-d511dbf/source/reference-spectrum.rs work/p10-reference-spectrum/src/main.rs
cargo build --release --manifest-path work/p10-reference-spectrum/Cargo.toml --locked
cargo clippy --release --manifest-path work/p10-reference-spectrum/Cargo.toml --locked -- -D warnings
RUSTDOCFLAGS='-D warnings' cargo doc --manifest-path work/p10-reference-spectrum/Cargo.toml --locked --no-deps
(ulimit -v 524288; timeout --signal=TERM --kill-after=5s 900s /usr/bin/time -v work/p10-reference-spectrum/target/release/p10-reference-spectrum 96 --run > work/p10-reference-spectrum/m96-frozen.stdout 2> work/p10-reference-spectrum/m96-frozen.time)
(ulimit -v 524288; timeout --signal=TERM --kill-after=5s 900s /usr/bin/time -v work/p10-reference-spectrum/target/release/p10-reference-spectrum 192 --run > work/p10-reference-spectrum/m192-frozen.stdout 2> work/p10-reference-spectrum/m192-frozen.time)
mkdir -p work/p10-force-snapshot-tool/src
cp evidence/p10/force-resolution-snapshot-forensics-d7336c8/source/comparator-Cargo.toml work/p10-force-snapshot-tool/Cargo.toml
cp evidence/p10/force-resolution-snapshot-forensics-d7336c8/source/comparator-Cargo.lock work/p10-force-snapshot-tool/Cargo.lock
cp evidence/p10/force-resolution-snapshot-forensics-d7336c8/source/comparator.rs work/p10-force-snapshot-tool/src/main.rs
FORENSICS_SOURCE=d7336c877880c28247c4658b6b9631554ebc4762 cargo build --release --manifest-path work/p10-force-snapshot-tool/Cargo.toml --locked
/usr/bin/time -v work/p10-force-snapshot-tool/target/release/p10-force-snapshot-tool compare work/p10-reference-spectrum/reference-m96-clock4096-n96.coeff.bin work/p10-reference-spectrum/reference-m192-clock4096-n96.coeff.bin 4096 96 96 96 192 1 1 90f5d917a12de43efe51ee51e84729c025d55b6809c1a6c1857b556eb1c52143 f9573bcd5de01f5378110c07103d2526a2b4c3307566ed8d9287babcc607bb3b > work/p10-reference-spectrum/compare-frozen.stdout 2> work/p10-reference-spectrum/compare-frozen.time
