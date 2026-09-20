#!/usr/bin/env python3
"""Deterministic build-input inventory for the staged release binary.

Hashes EVERY input that produces the staged solver: the harness sources,
build.rs, Cargo.toml and Cargo.lock, the FULL local dependency sources
(crates/nsbu-solver and crates/nsbu-benchmarks including their Cargo.toml
files) and the path-included observer source from the reviewed composite.
RUN_SOURCE is the SHA-256 over the sorted inventory lines; it is the exact
``source=`` field of the production profile identity.  Output paths are
relative to the repository root for reproducibility on any checkout.
"""

from __future__ import annotations

import hashlib
import json
import sys
from collections.abc import Iterable
from pathlib import Path

REPO = Path("/mnt/niva-array/nsbu-solver/work/opencode-n512-temporal-prep-20260914")
HARNESS = REPO / "evidence/p10/avx-scheduled-endpoint/harness"
LIST_OUT = REPO / "evidence/p10/n512-h32-sulaco-launch-20260914/docs/source-sha256.list"
LOCAL_CRATES = ("nsbu-solver", "nsbu-benchmarks")
PATH_INCLUDED = (
    REPO / "evidence/p10/avx-parallel-reduced-composite-7467e26/harness/src/observer.rs",
)


def sha256_file(path: Path, chunk: int = 1 << 20) -> str:
    digest = hashlib.sha256()
    with path.open("rb") as stream:
        for block in iter(lambda: stream.read(chunk), b""):
            digest.update(block)
    return digest.hexdigest()


def _without_target(paths: Iterable[Path]) -> set[Path]:
    return {p for p in paths if "target" not in p.parts and ".git" not in p.parts}


def inventory(repo: Path = REPO) -> list[Path]:
    harness = repo / "evidence/p10/avx-scheduled-endpoint/harness"
    paths = _without_target(harness.rglob("*.rs"))
    paths.update({harness / "Cargo.toml", harness / "Cargo.lock"})
    for crate in LOCAL_CRATES:
        root = repo / "crates" / crate
        if not root.is_dir():
            raise SystemExit(f"refused: local dependency source missing ({root})")
        paths.update(root.rglob("*.rs"))
        paths.update(root.rglob("*.toml"))
        # Data embedded via include_str! (e.g. the frozen
        # similarity-mms-v2.json CASE_DEFINITION) is a BUILD INPUT: without
        # it a clean-room checkout cannot compile at all, so it must be in
        # the inventory.  Test fixtures are included so `cargo test`
        # regeneration of identity fixtures reproduces from the same list.
        paths.update(root.rglob("*.json"))
    # The path-dependency crates inherit [workspace.package] fields and the
    # dependency lock from the REPOSITORY ROOT: the root Cargo.toml, the
    # workspace Cargo.lock and the pinned rust-toolchain.toml are therefore
    # build inputs as well (a clean-room checkout needs them to resolve).
    for rootfile in ("Cargo.toml", "Cargo.lock", "rust-toolchain.toml"):
        extra = repo / rootfile
        if not extra.is_file():
            raise SystemExit(f"refused: workspace root build input missing ({extra})")
        paths.add(extra)
    for extra in PATH_INCLUDED:
        if not extra.is_file():
            raise SystemExit(f"refused: path-included source missing ({extra})")
        paths.add(extra)
    missing = [path for path in paths if not path.is_file()]
    if missing:
        raise SystemExit(f"refused: inventory entries missing: {missing[:3]}")
    return sorted((path for path in paths if path.is_file()),
                  key=lambda path: str(path.relative_to(repo)))


def build_list(repo: Path = REPO) -> tuple[list[str], str]:
    lines = [f"{sha256_file(path)}  {path.relative_to(repo)}"
             for path in inventory(repo)]
    run_source = hashlib.sha256(("\n".join(lines) + "\n").encode("utf-8")).hexdigest()
    return lines, run_source


def main(argv: list[str]) -> int:
    write = "--write" in argv
    lines, run_source = build_list()
    if write:
        LIST_OUT.parent.mkdir(parents=True, exist_ok=True)
        LIST_OUT.write_text("\n".join(lines) + "\n")
    print(json.dumps({"entries": len(lines), "run_source": run_source,
                      "list": str(LIST_OUT), "written": write}))
    return 0


if __name__ == "__main__":
    raise SystemExit(main(sys.argv[1:]))
