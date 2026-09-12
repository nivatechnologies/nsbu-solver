#!/bin/sh
# Exact path-binding template used for the read-only early-clock and finer-state norm comparisons.
set -eu
T=work/p10-force-snapshot-tool/target/release/p10-force-snapshot-tool
O=work/p10-force-snapshot-forensics
B=/mnt/niva-array/nsbu-solver/work/p10-cached-m48-walkthrough/work/ignored-cached-m48-pilot/forensics
for clock in 64 128; do
  h0=$(sha256sum "$B/event-ordinary-branch0-clock$clock.coeff.bin" | cut -d' ' -f1)
  h1=$(sha256sum "$B/event-ordinary-branch1-clock$clock.coeff.bin" | cut -d' ' -f1)
  h2=$(sha256sum "$B/event-ordinary-branch2-clock$clock.coeff.bin" | cut -d' ' -f1)
  /usr/bin/time -v "$T" compare "$B/event-ordinary-branch0-clock$clock.coeff.bin" "$B/event-ordinary-branch1-clock$clock.coeff.bin" "$clock" 12 16 48 48 32 32 "$h0" "$h1"
  /usr/bin/time -v "$T" compare "$B/event-ordinary-branch1-clock$clock.coeff.bin" "$B/event-ordinary-branch2-clock$clock.coeff.bin" "$clock" 16 24 48 48 32 32 "$h1" "$h2"
done
# Exact-zero inputs were generated as checked snapshot-length byte arrays. Each state-norm call
# used compare ZERO STATE CLOCK N N 0 FORCE_M 0 32 ZERO_SHA STATE_SHA.
for n in 32 48; do
  (cd "/mnt/niva-array/nsbu-solver/work/p10-cli-m96-feasibility/work/m96-spatial-preflight/n$n" && env RUN_SOURCE=def4730b08025fdd06e7a8a0d78116aea24b6e2c cargo run --release --locked -- --dry-run)
done
