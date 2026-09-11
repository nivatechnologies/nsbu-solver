#!/usr/bin/env python3
"""Extract the final spectral full-band pairs from this one frozen Debug artifact.

This is deliberately a format-checked evidence extractor, not a production parser.
It makes no qualification or convergence determination.
"""
import gzip
import hashlib
import json
import math
import re
import sys
from pathlib import Path

ROOT = Path(__file__).parent
RAW = ROOT / "full.stdout.gz"
EXPECTED_SHA256 = "b15a60c9c47598db346488561935eefbcd146bd4a5460d315580d2383d5f7c5c"
PAIRS = [
    ("space_n8_to_n12", "0=CM N8 step16", "1=CM N12 step16"),
    ("space_n12_to_n16", "1=CM N12 step16", "2=CM N16 step16"),
    ("time_step64_to_step32", "3=CM N16 step64", "4=CM N16 step32"),
    ("time_step32_to_step16", "4=CM N16 step32", "2=CM N16 step16"),
    ("method_cm_to_ho", "2=CM N16 step16", "5=HO N16 step16"),
]


def balanced(text, start, opening, closing):
    if text[start] != opening:
        raise ValueError("delimiter missing")
    depth = 0
    for index in range(start, len(text)):
        if text[index] == opening:
            depth += 1
        elif text[index] == closing:
            depth -= 1
            if depth == 0:
                return text[start : index + 1]
    raise ValueError("unbalanced Debug payload")


def section(text, marker, opening="{"):
    index = text.index(marker) + len(marker)
    index = text.index(opening, index)
    return balanced(text, index, opening, "}" if opening == "{" else "]")


def main():
    raw = RAW.read_bytes()
    digest = hashlib.sha256(raw).hexdigest()
    if digest != EXPECTED_SHA256:
        raise ValueError(f"raw SHA256 mismatch: {digest}")
    text = gzip.decompress(raw).decode("utf-8")
    events = re.findall(r"^event=(\d+) .*? elapsed=(\d+) raw=", text, re.MULTILINE)
    expected_events = [(str(number), str(clock)) for number, clock in enumerate([0, 2047, 2048, 4095, 4096], 1)]
    if events != expected_events:
        raise ValueError(f"unexpected events: {events}")
    matches = list(re.finditer(r"^event=5 .*? elapsed=4096 raw=DiagnosticEvent ", text, re.MULTILINE))
    if len(matches) != 1:
        raise ValueError("final event is not unique")
    event = balanced(text, text.index("{", matches[0].end()), "{", "}")
    spectral = section(event, "spectral: RefinementSample ")
    comparisons = section(spectral, "comparisons: ", "[")
    entries = []
    cursor = 0
    while True:
        found = comparisons.find("BandComparison {", cursor)
        if found < 0:
            break
        entries.append(balanced(comparisons, found + len("BandComparison "), "{", "}"))
        cursor = found + 1
    if len(entries) != 5:
        raise ValueError(f"expected exactly 5 spectral comparisons, got {len(entries)}")
    pattern = re.compile(
        r"^\{ full: Norms \{ l2: ([^,]+), h1: ([^,]+), vorticity_l2: ([^,]+), divergence_l2: ([^ }]+) \}, common:"
    )
    pairs = []
    for (name, left, right), entry in zip(PAIRS, entries, strict=True):
        match = pattern.match(entry)
        if not match:
            raise ValueError("missing leading full Norms or unexpected comparison layout")
        norms = dict(zip(("l2", "h1", "vorticity_l2", "divergence_l2"), map(float, match.groups()), strict=True))
        if not all(math.isfinite(value) for value in norms.values()):
            raise ValueError("non-finite full norm")
        pairs.append({"name": name, "left": left, "right": right, "full_norms": norms})
    output = {
        "scope": "final event spectral RefinementSample comparisons only; full norms only; no convergence or qualification claim",
        "raw_sha256": digest,
        "event": 5,
        "elapsed_ticks": 4096,
        "pairs": pairs,
    }
    (ROOT / "spectral-full-norms.json").write_text(json.dumps(output, indent=2) + "\n")


if __name__ == "__main__":
    main()
