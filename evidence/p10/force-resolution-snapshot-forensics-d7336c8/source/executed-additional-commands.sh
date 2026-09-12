#!/bin/sh
# Executed from repository root. T and O match the ignored working paths retained in raw output.
set -eu
T=work/p10-force-snapshot-tool/target/release/p10-force-snapshot-tool
O=work/p10-force-snapshot-forensics
FORENSICS_SOURCE=d7336c877880c28247c4658b6b9631554ebc4762 cargo build --release --locked --manifest-path work/p10-force-snapshot-tool/Cargo.toml
/usr/bin/time -v "$T" compare "$O/snapshots/m24-w12-clock2048.coeff.bin" /mnt/niva-array/nsbu-solver/work/p10-cached-m48-walkthrough/work/ignored-cached-m48-pilot/forensics/event-ordinary-branch0-clock2048.coeff.bin 2048 12 12 24 48 12 32 ae1245a87c7e7a81ada6bb37fc0be8c620b5b358bb6a802e2e35633375ffb1ea 42b06216f27cc12121b3fb8e1203b98e4a877f5a0aa3fd9e8269a48bb089f015
/usr/bin/time -v "$T" compare /mnt/niva-array/nsbu-solver/work/p10-cached-m48-walkthrough/work/ignored-cached-m48-pilot/forensics/event-ordinary-branch0-clock2048.coeff.bin /mnt/niva-array/nsbu-solver/work/p10-cli-m96-feasibility/work/m96-cli-short/full-forensics/forensics/accepted-n12-m96-cm16-clock2048.coeff.bin 2048 12 12 48 96 32 32 42b06216f27cc12121b3fb8e1203b98e4a877f5a0aa3fd9e8269a48bb089f015 22d3e26aeacf7089236c2dd1db490b91aeec336a0eb13dd328ec7b35aeb5d38e
/usr/bin/time -v "$T" compare "$O/snapshots/m24-w12-clock2048.coeff.bin" /mnt/niva-array/nsbu-solver/work/p10-cli-m96-feasibility/work/m96-cli-short/full-forensics/forensics/accepted-n12-m96-cm16-clock2048.coeff.bin 2048 12 12 24 96 12 32 ae1245a87c7e7a81ada6bb37fc0be8c620b5b358bb6a802e2e35633375ffb1ea 22d3e26aeacf7089236c2dd1db490b91aeec336a0eb13dd328ec7b35aeb5d38e
/usr/bin/time -v python3 work/p10-force-snapshot-forensics/validate_production.py
/usr/bin/time -v cargo fmt --manifest-path work/p10-force-snapshot-tool/Cargo.toml -- --check
/usr/bin/time -v env FORENSICS_SOURCE=d7336c877880c28247c4658b6b9631554ebc4762 cargo clippy --release --locked --manifest-path work/p10-force-snapshot-tool/Cargo.toml --all-targets -- -D warnings
/usr/bin/time -v env FORENSICS_SOURCE=d7336c877880c28247c4658b6b9631554ebc4762 RUSTDOCFLAGS='-D warnings' cargo doc --release --locked --manifest-path work/p10-force-snapshot-tool/Cargo.toml --no-deps
/mnt/niva-array/nsbu-solver/work/rust-tools/bin/rust-code-analysis-cli -m -O json -p work/p10-force-snapshot-tool/src
M48=/mnt/niva-array/nsbu-solver/work/p10-cached-m48-walkthrough/work/ignored-cached-m48-pilot/forensics
for clock in 2048 4096; do
  for branch in 0 1 2 3 4 5; do
    cmp "$M48/event-ordinary-branch${branch}-clock${clock}.coeff.bin" "$M48/event-probe-branch${branch}-clock${clock}.coeff.bin"
    sha256sum "$M48/event-ordinary-branch${branch}-clock${clock}.coeff.bin" "$M48/event-probe-branch${branch}-clock${clock}.coeff.bin"
  done
done
/usr/bin/time -v "$T" compare /mnt/niva-array/nsbu-solver/work/p10-cached-m48-walkthrough/work/ignored-cached-m48-pilot/forensics/event-ordinary-branch0-clock4096.coeff.bin /mnt/niva-array/nsbu-solver/work/p10-cli-m96-feasibility/work/m96-cli-short/full-forensics/forensics/accepted-n12-m96-cm16-clock4096.coeff.bin 4096 12 12 48 96 32 32 1afa69189063eca579f2e234096b11ece162b7c45e2f4768dc22a2c55ecd6b8d aeeaa86eca750d47a46fd787dbd32229c4f7ae2761ddb5fdd05c782190f33758
/usr/bin/time -v "$T" compare "$O/snapshots/m24-w12-clock4096.coeff.bin" /mnt/niva-array/nsbu-solver/work/p10-cli-m96-feasibility/work/m96-cli-short/full-forensics/forensics/accepted-n12-m96-cm16-clock4096.coeff.bin 4096 12 12 24 96 12 32 b70d219d54b3a58d0e53fd955e54b424176d306470d0ede24471c754a5c33c51 aeeaa86eca750d47a46fd787dbd32229c4f7ae2761ddb5fdd05c782190f33758
/usr/bin/time -v "$T" compare /mnt/niva-array/nsbu-solver/work/p10-cli-m96-feasibility/work/m96-cli-short/full-forensics/forensics/accepted-n12-m96-cm16-clock2048.coeff.bin /mnt/niva-array/nsbu-solver/work/p10-cli-m96-feasibility/work/m96-spatial-controls/n16/forensics/accepted-n16-m96-cm16-clock2048.coeff.bin 2048 12 16 96 96 32 32 22d3e26aeacf7089236c2dd1db490b91aeec336a0eb13dd328ec7b35aeb5d38e 6416422305be6b222b189761dc366684009ad879fff3ff8b6ab6227b51ac9632
/usr/bin/time -v "$T" compare /mnt/niva-array/nsbu-solver/work/p10-cli-m96-feasibility/work/m96-spatial-controls/n16/forensics/accepted-n16-m96-cm16-clock2048.coeff.bin /mnt/niva-array/nsbu-solver/work/p10-cli-m96-feasibility/work/m96-spatial-controls/n24/forensics/accepted-n24-m96-cm16-clock2048.coeff.bin 2048 16 24 96 96 32 32 6416422305be6b222b189761dc366684009ad879fff3ff8b6ab6227b51ac9632 eef26edc20f0d98ffb649a0e0b65831b2ec188dcc8b50629658ac0b0d4ccb19e
