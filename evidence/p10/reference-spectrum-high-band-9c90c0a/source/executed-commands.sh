#!/bin/sh
# Actual working directory: repository root at 9c90c0a796f9583c2d165bdca44f93406f179d48.
# Restore the two ignored harness packages from tracked source.
mkdir -p work/p10-reference-spectrum-hi/src work/p10-force-snapshot-tool-hi/src work/p10-reference-spectrum
cp evidence/p10/reference-spectrum-high-band-9c90c0a/source/reference-Cargo.toml work/p10-reference-spectrum-hi/Cargo.toml
cp evidence/p10/reference-spectrum-high-band-9c90c0a/source/reference-Cargo.lock work/p10-reference-spectrum-hi/Cargo.lock
cp evidence/p10/reference-spectrum-high-band-9c90c0a/source/reference-spectrum-hi.rs work/p10-reference-spectrum-hi/src/main.rs
cp evidence/p10/reference-spectrum-high-band-9c90c0a/source/comparator-Cargo.toml work/p10-force-snapshot-tool-hi/Cargo.toml
cp evidence/p10/reference-spectrum-high-band-9c90c0a/source/comparator-Cargo.lock work/p10-force-snapshot-tool-hi/Cargo.lock
cp evidence/p10/reference-spectrum-high-band-9c90c0a/source/comparator-hi.rs work/p10-force-snapshot-tool-hi/src/main.rs
cargo build --release --manifest-path work/p10-reference-spectrum-hi/Cargo.toml --locked
FORENSICS_SOURCE=9c90c0a796f9583c2d165bdca44f93406f179d48 cargo build --release --manifest-path work/p10-force-snapshot-tool-hi/Cargo.toml --locked
# Generation preflight and frozen runs used a 4 GiB per-process RLIMIT_AS.
(ulimit -v 4194304; timeout --signal=TERM --kill-after=5s 300s /usr/bin/time -v work/p10-reference-spectrum-hi/target/release/p10-reference-spectrum-hi 384 --preflight > work/p10-reference-spectrum-hi/m384-preflight.stdout 2> work/p10-reference-spectrum-hi/m384-preflight.time)
(ulimit -v 4194304; timeout --signal=TERM --kill-after=5s 120s /usr/bin/time -v work/p10-reference-spectrum-hi/target/release/p10-reference-spectrum-hi 192 --run > work/p10-reference-spectrum-hi/m192-run.stdout 2> work/p10-reference-spectrum-hi/m192-run.time)
(ulimit -v 4194304; timeout --signal=TERM --kill-after=5s 480s /usr/bin/time -v work/p10-reference-spectrum-hi/target/release/p10-reference-spectrum-hi 384 --run > work/p10-reference-spectrum-hi/m384-run.stdout 2> work/p10-reference-spectrum-hi/m384-run.time)
# The generator writes both N192 files to work/p10-reference-spectrum/ by design.
# Separate comparison used a 2 GiB per-process RLIMIT_AS and 60 s timeout.
(ulimit -v 2097152; timeout --signal=TERM --kill-after=5s 60s /usr/bin/time -v work/p10-force-snapshot-tool-hi/target/release/p10-force-snapshot-tool-hi compare work/p10-reference-spectrum/reference-m192-clock4096-n192.coeff.bin work/p10-reference-spectrum/reference-m384-clock4096-n192.coeff.bin 4096 192 192 192 384 1 1 3300a791a3b95ca40ca32698d1217d63064183dc937e927693d4226d29445886 519bf90eafacf4ae4f88779f22d6780516d81edc831a5bb546a89fde31842aa6 > work/p10-reference-spectrum-hi/compare-m192-m384.stdout 2> work/p10-reference-spectrum-hi/compare-m192-m384.time)
