#!/usr/bin/env bash
set -uo pipefail
inner=$1
outer=$2
exec timeout --signal=TERM --kill-after=10s "${outer}s" /usr/bin/time -v "target/release/p10-m96-next-n64-forensics" --run --deadline-seconds "$inner"
