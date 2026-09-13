#!/bin/sh
set -eu
[ "$#" -eq 4 ] || exit 64
output=$1
partial=$2
archive=$3
logs=$4
mkdir "$partial"
(cd "$output" && find . -type f -print0 | LC_ALL=C sort -z | xargs -0 sha256sum) >"$logs/source.sha256"
cp -a "$output/." "$partial/"
(cd "$partial" && find . -type f -print0 | LC_ALL=C sort -z | xargs -0 sha256sum) >"$logs/archive.sha256"
cmp "$logs/source.sha256" "$logs/archive.sha256"
sync "$partial"
mv "$partial" "$archive"
printf 'archive=%s files=%s bytes=%s\n' "$archive" "$(find "$archive" -type f | wc -l)" "$(du -sb "$archive" | awk '{print $1}')" >"$logs/archive-receipt.txt"
