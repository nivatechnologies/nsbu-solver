"""Measure per-function CRAP from Radon/coverage.py or RCA/LLVM reports."""
import argparse
import json
from pathlib import Path
from typing import Literal, cast

Json = None | bool | int | float | str | list['Json'] | dict[str, 'Json']


def obj(value: Json) -> dict[str, Json]:
    """Require an object rather than silently accepting malformed evidence."""
    if not isinstance(value, dict):
        raise ValueError('expected object')
    return value


def seq(value: Json) -> list[Json]:
    """Require an array."""
    if not isinstance(value, list):
        raise ValueError('expected array')
    return value


def number(value: Json) -> float:
    """Require a finite numeric field."""
    import math
    if isinstance(value, bool) or not isinstance(value, (int, float)):
        raise ValueError('expected number')
    if not math.isfinite(value):
        raise ValueError('nonfinite number')
    return float(value)


def read(path: Path) -> Json:
    """Load one JSON document; callers narrow its structure."""
    return cast(Json, json.loads(path.read_text()))


def fraction(covered: float, total: float) -> float:
    """Return a checked measurement fraction (empty scope is fully covered)."""
    if covered < 0 or total < covered:
        raise ValueError('invalid coverage counts')
    return covered / total if total else 1.0


def score(complexity: float, coverage: float) -> float:
    """CRAP formula with coverage in [0, 1]."""
    if complexity < 1 or not 0 <= coverage <= 1:
        raise ValueError('invalid function measurement')
    return complexity**2 * (1 - coverage)**3 + complexity


def result(path: str, function: str, cc: float, coverage: float) -> dict[str, Json]:
    """Preserve the inputs to each score."""
    return dict(path=path, function=function, cyclomatic=cc,
                branch_coverage_fraction=coverage, crap=score(cc, coverage))


def python_functions(blocks: list[Json]) -> list[dict[str, Json]]:
    """Flatten classes and nested functions in Radon's report."""
    found: list[dict[str, Json]] = []
    for entry in blocks:
        block = obj(entry)
        if block['type'] != 'class':
            found.append(block)
        for key in ('methods', 'closures'):
            found.extend(python_functions(seq(block.get(key, []))))
    return found


def python_branch_outcomes(measured: dict[str, Json], start: float,
                           end: float) -> list[float]:
    """Collect covered and missing outcomes whose origin belongs to a function."""
    outcomes: list[float] = []
    for key, covered in [('executed_branches', 1.0), ('missing_branches', 0.0)]:
        for branch in seq(measured[key]):
            if start <= number(seq(branch)[0]) <= end:
                outcomes.append(covered)
    return outcomes


def python_coverage(measured: dict[str, Json], start: float, end: float) -> float:
    """Measure branches, or observable body execution for branchless functions."""
    outcomes = python_branch_outcomes(measured, start, end)
    if outcomes:
        return fraction(sum(outcomes), len(outcomes))
    executed = seq(measured['executed_lines'])
    # Definition execution alone does not count as body execution.
    return float(any(start < number(line) <= end for line in executed))


def python_row(path: str, measured: dict[str, Json], fn: dict[str, Json]) -> dict[str, Json]:
    """Measure one Radon function against its coverage.py file entry."""
    start, end = number(fn['lineno']), number(fn['endline'])
    return result(path, str(fn['name']), number(fn['complexity']),
                  python_coverage(measured, start, end))


def python_report(metrics: Json, coverage: Json) -> list[dict[str, Json]]:
    """Use branch origins within each function's inclusive source span."""
    files = obj(obj(coverage)['files'])
    rows: list[dict[str, Json]] = []
    for path, entries in obj(metrics).items():
        measured = obj(files[path])
        for fn in python_functions(seq(entries)):
            rows.append(python_row(path, measured, fn))
    return rows


def rust_functions(space: dict[str, Json]) -> list[dict[str, Json]]:
    """Retain each function and its nested-closure inclusive complexity."""
    found = [space] if space['kind'] == 'function' else []
    for child in seq(space['spaces']):
        found.extend(rust_functions(obj(child)))
    return found


def rust_coverage(file: dict[str, Json], start: float, end: float) -> float:
    """Merge repeated generic/profile branch coordinates by observed outcomes."""
    branches: dict[tuple[float, ...], list[bool]] = {}
    for item in seq(file['branches']):
        branch = seq(item)
        if start <= number(branch[0]) <= end:
            key = tuple(number(v) for v in branch[:4])
            seen = branches.setdefault(key, [False, False])
            seen[0] |= number(branch[4]) > 0
            seen[1] |= number(branch[5]) > 0
    if branches:
        return fraction(sum(sum(seen) for seen in branches.values()), 2 * len(branches))
    segments = [seq(segment) for segment in seq(file['segments'])]
    return float(any(start <= number(s[0]) <= end and s[3] is True
                     and number(s[2]) > 0 for s in segments))


def rust_report(metrics: Path, coverage: Json) -> list[dict[str, Json]]:
    """Match RCA source spans to the corresponding LLVM source file."""
    data = obj(seq(obj(coverage)['data'])[0])
    files = [obj(file) for file in seq(data['files'])]
    rows: list[dict[str, Json]] = []
    for line in metrics.read_text().splitlines():
        unit = obj(cast(Json, json.loads(line)))
        path = str(unit['name'])
        matches = [f for f in files if str(f['filename']).endswith('/' + path)
                   or str(f['filename']) == path]
        functions = rust_functions(unit)
        if not functions:
            continue
        if len(matches) != 1:
            raise ValueError(f'coverage file match is ambiguous or missing: {path}')
        for fn in functions:
            cc = number(obj(obj(fn['metrics'])['cyclomatic'])['sum'])
            cov = rust_coverage(matches[0], number(fn['start_line']), number(fn['end_line']))
            rows.append(result(path, str(fn['name']), cc, cov))
    return rows


def main() -> None:
    """Write measured scores and fail if any function reaches CRAP 25."""
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument('language', choices=['python', 'rust'])
    parser.add_argument('metrics', type=Path)
    parser.add_argument('coverage', type=Path)
    parser.add_argument('output', type=Path)
    args = parser.parse_args()
    language = cast(Literal['python', 'rust'], args.language)
    metrics = cast(Path, args.metrics)
    coverage_path = cast(Path, args.coverage)
    output = cast(Path, args.output)
    coverage = read(coverage_path)
    rows = (python_report(read(metrics), coverage) if language == 'python'
            else rust_report(metrics, coverage))
    if not rows:
        raise ValueError('no functions measured')
    maximum = max(number(row['crap']) for row in rows)
    report = cast(dict[str, Json], dict(
        formula='CC^2 * (1 - branch_coverage)^3 + CC',
        branchless='observed executable body coverage', functions=rows,
        maximum=maximum,
    ))
    output.write_text(json.dumps(report, indent=2) + '\n')
    if maximum >= 25:
        raise SystemExit('per-function CRAP must be <25; see output report')


if __name__ == '__main__':
    main()
