"""Recompute derived timing rows from the preserved GNU time stderr files."""
from __future__ import annotations
import json
import re
from pathlib import Path

HERE = Path(__file__).resolve().parent
NAMES = (
    "cm-direct-1", "cm-direct-2", "cm-cached-1", "cm-cached-2",
    "ho-direct-1", "ho-direct-2", "ho-cached-1", "ho-cached-2",
)
EXCLUDED = ("actual_charged_work", "current_attempt_cache_work", "integration_force_policy")

def elapsed_seconds(value: str) -> float:
    """Parse GNU time h:mm:ss, m:ss, or seconds without a minute error."""
    value = value.strip()
    seconds = 0.0
    for part in value.split(":"):
        seconds = seconds * 60.0 + float(part)
    return seconds

def elapsed_from_stderr(path: Path) -> float:
    match = re.search(r"^\s*Elapsed \(wall clock\) time \(h:mm:ss or m:ss\): (.+)$", path.read_text(), re.MULTILINE)
    if not match:
        raise ValueError(f"missing GNU elapsed line in {path}")
    # The final ': ' is the delimiter; the value itself may contain colons.
    return elapsed_seconds(match.group(1))

def without_excluded(value):
    if isinstance(value, dict):
        return {k: without_excluded(v) for k, v in value.items() if k not in EXCLUDED}
    if isinstance(value, list):
        return [without_excluded(v) for v in value]
    return value

def main() -> None:
    old = json.loads((HERE / "summary.json").read_text())
    runs = []
    for name in NAMES:
        data = json.loads((HERE / f"{name}.json").read_text())
        method, cache, rep = name.split("-")
        run = {
            "run": name, "method": method, "cache": cache, "rep": int(rep),
            "wall_seconds": elapsed_from_stderr(HERE / f"{name}.stderr"),
            "user_seconds": data.get("_unused", None),
        }
        stderr = (HERE / f"{name}.stderr").read_text()
        run["user_seconds"] = float(re.search(r"User time \(seconds\): ([0-9.]+)", stderr).group(1))
        run["system_seconds"] = float(re.search(r"System time \(seconds\): ([0-9.]+)", stderr).group(1))
        run["max_rss_kib"] = int(re.search(r"Maximum resident set size \(kbytes\): ([0-9]+)", stderr).group(1))
        run["exit"] = int((HERE / f"{name}-exit.txt").read_text().strip())
        run["clock"] = data["clock"]["actual_elapsed_ticks"]
        run["attempts"] = data["attempts"]
        runs.append(run)
    old["runs"] = runs
    med = {}
    for method in ("cm", "ho"):
        direct = sorted(r["wall_seconds"] for r in runs if r["method"] == method and r["cache"] == "direct")
        cached = sorted(r["wall_seconds"] for r in runs if r["method"] == method and r["cache"] == "cached")
        dm = sum(direct) / 2
        cm = sum(cached) / 2
        med[method] = {"direct_wall_median_seconds": dm, "cached_wall_median_seconds": cm, "direct_to_cached_ratio": dm / cm}
    old["median_speed"] = med
    old["analysis"] = {
        "parser": "analyze_timing.py: take text after final ': ', then fold parts as seconds = seconds*60 + part",
        "correction": "Derived wall seconds were recomputed from preserved stderr; raw timing files were not rerun or modified.",
    }
    # Recheck the documented paired core comparison from the preserved JSON.
    a = json.loads((HERE / "cm-direct-1.json").read_text())
    b = json.loads((HERE / "cm-cached-1.json").read_text())
    old["paired_comparison"]["core_diagnostic_json_equal"] = without_excluded(a) == without_excluded(b)
    (HERE / "summary.json").write_text(json.dumps(old, indent=2) + "\n")

if __name__ == "__main__":
    main()
