"""Check NSBU's public bootstrap and immutable review inputs. Standard library only.

This checks packaging, not a Rust implementation or a PDE trajectory. Run from
any directory; paths default to the checkout containing this script.
"""

from __future__ import annotations

import argparse
import hashlib
import json
import os
from pathlib import Path, PurePosixPath
import re
import sys
from typing import TypedDict, NotRequired
if not __package__:
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from tools.json_types import JsonObject, decode, object_value, array_value, string_value
from urllib.parse import unquote, urlsplit


MANIFEST_SHA256 = "4393e8766fe00cd4f3d92452479ac6fe1680d08319b05c9b7f1e4a3908fd2297"
FROZEN_SHA256 = {
    "COMPLETE_DESIGN.md": "fabca082cf73fee5f64c7f67800308b5115936bbec04838100a0d2ee425b81d9",
    "similarity-mms-v2.json": "e1236f7b3c51537acd17381402ca420ba7872a7b9dbc64b2f0d9d5108a468f7e",
}
PROBLEM_SHA256 = "ba81b7709e68cb118d3fc60c1d9bbcd27f424358b121ed4be08660ab8b88210f"
COMPONENTS = (
    "ADVERSARIAL_REVIEW.md", "COMPLETE_DESIGN.md", "CONSTRUCTION_LEDGER.md",
    "IMPLEMENTATION_PLAN.md", "SOURCE_FEASIBILITY.md",
    "navier-runtime-adversarial-review-prompt.md",
    "navier-runtime-construction-design.md", "navier-runtime-design.md",
    "navier-runtime-verification.py", "navier-runtime-verification-results.json",
    "similarity-mms-v2.json", "source-feasibility-policy.json",
)
REQUIRED = (
    "README.md", "LICENSE", "NOTICE", "THIRD_PARTY.md", "CONTRIBUTING.md",
    "IMPLEMENTATION_PLAN.md", "requirements-dev.txt", "project-status.json",
    ".gitignore", ".gitattributes", ".github/workflows/checks.yml",
    ".github/pull_request_template.md", "docs/INSTALL.md", "docs/USAGE.md",
    "docs/SCIENTIFIC_SCOPE.md", "docs/PROVENANCE.md",
    "benchmarks/similarity-mms-v2.json", "tools/check_repository.py",
    "tools/verify_design.py", "tools/tests/test_bootstrap_tools.py",
    "docs/design/navier-runtime-review-manifest.json",
    "docs/design/navier-runtime-review-packet.md",
) + tuple(f"docs/design/{name}" for name in COMPONENTS)
IGNORED_DIRS = {
    ".git", ".venv", "work", "target", "runs", "checkpoints", "__pycache__",
    ".pytest_cache", ".mypy_cache", ".ruff_cache", "mutants", ".complexipy_cache",
}
IGNORED_FILES = {".DS_Store"}
TEXT_SUFFIXES = {".md", ".py", ".json", ".yml", ".yaml", ".toml", ".txt", ".rs"}
PRIVATE_PATH = re.compile(r"/(?:Users|home)/[^\s/]+/")
INLINE_LINK = re.compile(r"!?\[[^\]\n]*\]\(\s*(<[^>\n]+>|[^\s)]+)(?:\s+[\"'][^\n]*?[\"'])?\s*\)")
REFERENCE_LINK = re.compile(r"^\s{0,3}\[[^\]\n]+\]:\s*(<[^>\n]+>|\S+)", re.MULTILINE)
FENCE = re.compile(r"^\s{0,3}(`{3,}|~{3,})(.*)$")


def sha256(path: Path) -> str:
    """Hash the actual file bytes, including preserved line endings."""
    return hashlib.sha256(path.read_bytes()).hexdigest()


def check_review(root: Path) -> JsonObject:
    """Validate the original manifest before trusting its component list."""
    design = root / "docs/design"
    manifest_path = design / "navier-runtime-review-manifest.json"
    if sha256(manifest_path) != MANIFEST_SHA256:
        raise ValueError("Original review manifest SHA-256 mismatch")
    manifest = object_value(decode(manifest_path.read_text(encoding="utf-8")))
    entries = [object_value(v) for v in array_value(manifest["components"])]
    if len(entries) != len(COMPONENTS) or {string_value(e["file"]) for e in entries} != set(COMPONENTS):
        raise ValueError("Original review must contain exactly the 12 public components")
    for entry in [*entries, object_value(manifest["packet"]) ]:
        name = string_value(entry["file"])
        if PurePosixPath(name).name != name:
            raise ValueError("Review manifest contains a nonlocal component name")
        path = design / name
        if path.stat().st_size != entry["bytes"] or sha256(path) != entry["sha256"]:
            raise ValueError(f"Preserved component differs from the reviewed bytes: {name}")
    for name, expected in FROZEN_SHA256.items():
        if sha256(design / name) != expected:
            raise ValueError(f"Frozen baseline SHA-256 mismatch: {name}")
    return {"components_verified": len(entries), "packet_verified": True,
            "manifest_sha256": MANIFEST_SHA256, "baseline_sha256": FROZEN_SHA256}


def check_case(root: Path) -> JsonObject:
    """Check both the complete artifact and its independently canonicalized input."""
    frozen = root / "docs/design/similarity-mms-v2.json"
    public = root / "benchmarks/similarity-mms-v2.json"
    if public.read_bytes() != frozen.read_bytes():
        raise ValueError("Benchmark copy differs from frozen similarity-mms-v2.json")
    case = object_value(decode(public.read_text(encoding="utf-8")))
    canonical = json.dumps(case["mathematical_problem"], sort_keys=True,
                           separators=(",", ":"), ensure_ascii=True).encode("ascii")
    digest = hashlib.sha256(canonical).hexdigest()
    if digest != PROBLEM_SHA256 or digest != object_value(case["problem_identity"])["sha256"]:
        raise ValueError("Mathematical problem identity mismatch")
    if case["source_instance"] or case["verified_pde_convergence"]:
        raise ValueError("Frozen specification must not claim source admission or PDE convergence")
    return {"benchmark_copy_identical": True, "problem_sha256": digest}


def public_files(root: Path) -> list[Path]:
    """Enumerate source files without traversing caches, run output or symlinks."""
    result: list[Path] = []
    for directory, dirs, files in os.walk(root, followlinks=False):
        base = Path(directory)
        dirs[:] = sorted(name for name in dirs if name not in IGNORED_DIRS)
        for name in [*dirs, *files]:
            path = base / name
            if path.is_symlink():
                raise ValueError(f"Public bootstrap must not contain a symlink: {path.relative_to(root)}")
        result.extend(base / name for name in sorted(files) if name not in IGNORED_FILES)
    return result


def unfenced_lines(text: str) -> str:
    """Ignore fenced code, including four-backtick blocks in the review packet."""
    output: list[str] = []
    opening = ""
    for line in text.splitlines():
        match = FENCE.match(line)
        if match:
            marker, rest = match.groups()
            if not opening:
                opening = marker
            elif marker[0] == opening[0] and len(marker) >= len(opening) and not rest.strip():
                opening = ""
            continue
        if not opening:
            output.append(line)
    if opening:
        raise ValueError("Unclosed Markdown code fence")
    return "\n".join(output)


def check_links(root: Path, path: Path, text: str) -> int:
    """Check local Markdown file targets, not remote URLs or heading anchors."""
    prose = unfenced_lines(text)
    targets = [m.group(1) for pattern in (INLINE_LINK, REFERENCE_LINK)
               for m in pattern.finditer(prose)]
    checked = 0
    for raw in targets:
        target = urlsplit(raw.strip("<>"))
        if target.scheme or target.netloc or not target.path:
            continue
        relative = Path(unquote(target.path))
        resolved = (path.parent / relative).resolve()
        if relative.is_absolute() or not resolved.is_relative_to(root):
            raise ValueError(f"Local Markdown link escapes checkout: {path.relative_to(root)}")
        if not resolved.exists():
            raise ValueError(f"Missing local Markdown target in {path.relative_to(root)}: {raw}")
        checked += 1
    return checked


def check_public_content(root: Path) -> JsonObject:
    """Catch common accidental private inclusions; this is not a secret scanner."""
    paths = public_files(root)
    local_links = 0
    markdown_files = 0
    for path in paths:
        name = path.name.lower()
        private_name = name.startswith(".env") and name != ".env.example"
        if private_name or name.endswith((".pem", ".key", "niva-adapter.md")):
            raise ValueError(f"Private material in public source: {path.relative_to(root)}")
        if path.suffix.lower() not in TEXT_SUFFIXES and name not in {"license", "notice"}:
            continue
        text = path.read_text(encoding="utf-8")
        if PRIVATE_PATH.search(text):
            raise ValueError(f"Personal absolute path in public text: {path.relative_to(root)}")
        if path.suffix.lower() == ".md":
            local_links += check_links(root, path, text)
            markdown_files += 1
    return {"public_files_scanned": len(paths), "markdown_files_checked": markdown_files,
            "local_file_links_checked": local_links, "private_inclusion_screen": "passed"}


def check_metadata(root: Path) -> JsonObject:
    status = object_value(decode((root / "project-status.json").read_text(encoding="utf-8")))
    if status.get("project") != "NSBU Solver" or status.get("license") != "Apache-2.0":
        raise ValueError("Project naming or licensing metadata differs from the adopted decision")
    if status.get("niva_dependency") is not False:
        raise ValueError("Public project must declare niva_dependency=false")
    license_text = (root / "LICENSE").read_text(encoding="utf-8")
    for marker in ("Apache License", "Version 2.0, January 2004", "Grant of Patent License",
                   "Accepting Warranty or Additional Liability", "END OF TERMS AND CONDITIONS"):
        if marker not in license_text:
            raise ValueError("LICENSE is missing an expected Apache-2.0 section")
    return {"project": status["project"], "license": status["license"],
            "private_dependency_declared": False}


class RepositoryReport(TypedDict):
    schema_version: int
    status: str
    scope: str
    checks: dict[str, JsonObject]
    errors: list[str]
    missing_files: NotRequired[list[str]]


def check_repository(root: Path) -> RepositoryReport:
    """Return a structured report; all failures stay active under Python -O."""
    root = root.resolve()
    report: RepositoryReport = {"schema_version": 1, "status": "passed", "scope": "bootstrap packaging only",
              "checks": {}, "errors": []}
    missing = [name for name in REQUIRED if not (root / name).is_file()]
    if missing:
        report.update(status="failed", missing_files=missing)
        report["errors"].append("Required bootstrap files are missing")
        return report
    for name, check in (("review", check_review), ("case", check_case),
                        ("metadata", check_metadata), ("public_content", check_public_content)):
        try:
            report["checks"][name] = check(root)
        except (OSError, UnicodeError, ValueError, KeyError, TypeError) as error:
            report["status"] = "failed"
            report["errors"].append(f"{name}: {str(error).replace(str(root), '<repo>')}")
    return report


class Arguments(argparse.Namespace):
    root: Path


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1],
                        help="Checkout root; defaults to this script's parent checkout")
    args = Arguments()
    parser.parse_args(namespace=args)
    report = check_repository(args.root)
    print(json.dumps(report, indent=2))
    return 0 if report["status"] == "passed" else 1


if __name__ == "__main__":
    sys.exit(main())
