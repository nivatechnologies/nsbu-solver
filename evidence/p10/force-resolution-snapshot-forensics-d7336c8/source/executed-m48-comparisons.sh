#!/bin/sh
set -eu
T=work/p10-force-snapshot-tool/target/release/p10-force-snapshot-tool
O=work/p10-force-snapshot-forensics
B=/mnt/niva-array/nsbu-solver/work/p10-cached-m48-walkthrough/work/ignored-cached-m48-pilot/forensics
run() { name=$1; shift; /usr/bin/time -v "$T" "$@" >"$O/$name.stdout" 2>"$O/$name.stderr"; }
run compare-m24-m48-4096 compare "$O/snapshots/m24-w12-clock4096.coeff.bin" "$B/event-ordinary-branch0-clock4096.coeff.bin" 4096 12 12 24 48 12 32 b70d219d54b3a58d0e53fd955e54b424176d306470d0ede24471c754a5c33c51 1afa69189063eca579f2e234096b11ece162b7c45e2f4768dc22a2c55ecd6b8d
for clock in 2048 4096; do
 case $clock in 2048) h0=42b06216f27cc12121b3fb8e1203b98e4a877f5a0aa3fd9e8269a48bb089f015; h1=34ee0dec2b18bb4b78da71a8496a27c740a926109099a92ef2fe99b2d55f65eb; h2=754888cd5b51b8b5705c25e850a68372124719ef32000c8c97ecbe51b913d972;; 4096) h0=1afa69189063eca579f2e234096b11ece162b7c45e2f4768dc22a2c55ecd6b8d; h1=59cc36292d8eab40244f5dd84db89c382668bce2cdbc5056ab1c7d89f2faa099; h2=67196daebf9c3d8dbedd1e48594ade181d34681fe4cb53806337560077e37567;; esac
 run compare-m48-n12-n16-$clock compare "$B/event-ordinary-branch0-clock$clock.coeff.bin" "$B/event-ordinary-branch1-clock$clock.coeff.bin" $clock 12 16 48 48 32 32 "$h0" "$h1"
 run compare-m48-n16-n24-$clock compare "$B/event-ordinary-branch1-clock$clock.coeff.bin" "$B/event-ordinary-branch2-clock$clock.coeff.bin" $clock 16 24 48 48 32 32 "$h1" "$h2"
done
