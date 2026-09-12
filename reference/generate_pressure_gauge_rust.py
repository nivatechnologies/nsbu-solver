"""Generate Rust gauge metadata from authoritative pressure-mean JSON files."""

import argparse
from collections.abc import Mapping, Sequence
import hashlib
from pathlib import Path
from typing import cast
from tools.json_types import Json, array_value, decode, object_value, string_value

CASE_SHA256 = "e1236f7b3c51537acd17381402ca420ba7872a7b9dbc64b2f0d9d5108a468f7e"
TIME_TO_TICKS = {"0": 0, "1/16384": 64, "1/8192": 128, "1/512": 2048, "1/256": 4096}
PROFILE_FIELDS: dict[str, Json] = {
    "classification": "EmpiricalQuadrature; conservative Python planning",
    "domain": "centered unit periodic cube",
    "support_radius": "21/50",
    "radial_rule": "exact Gaussian core plus Simpson cutoff collar",
    "accepted_pde_windows": 0,
}


def integer(value: Json) -> int:
    """Decode an integer while rejecting booleans and floating-point values."""
    if not isinstance(value, int) or isinstance(value, bool):
        raise ValueError("Expected a JSON integer")
    return value


def optional_string(value: Json) -> str | None:
    """Decode a nullable decimal string."""
    return None if value is None else string_value(value)


def rust_string(value: str) -> str:
    """Use a raw Rust literal for arbitrary decimal strings."""
    return f'r#"{value}"#'


def option(value: str | None) -> str:
    """Render an optional decimal string without rounding."""
    return "None" if value is None else f"Some({rust_string(value)})"


def read_report(path: Path) -> tuple[bytes, dict[str, Json]]:
    """Decode the complete producer shape before emitting any output."""
    raw = path.read_bytes()
    report = dict(object_value(decode(raw.decode())))
    estimates = array_value(report.get("estimates"))
    if (
        string_value(report.get("status")) != "pressure-mean-diagnostic-complete"
        or len(estimates) != 10
    ):
        raise ValueError(f"{path}: require one complete ten-estimate diagnostic")
    validate_report(path, report)
    return raw, report


def validate_report(path: Path, report: dict[str, Json]) -> None:
    """Require the complete fixed producer profile and exact clock metadata."""
    if string_value(report.get("case_sha256")) != CASE_SHA256:
        raise ValueError(f"{path}: frozen exact-v2 case hash mismatch")
    profiles = [object_value(item) for item in array_value(report.get("profiles"))]
    if len(profiles) != 10:
        raise ValueError(f"{path}: require ten declared profiles")
    time = string_value(profiles[0].get("time"))
    if time not in TIME_TO_TICKS:
        raise ValueError(f"{path}: unsupported exact clock")
    expected = expected_profiles(profiles)
    estimates = [object_value(item) for item in array_value(report.get("estimates"))]
    totals = [
        validate_profile(path, profile, estimate, time, declared)
        for profile, estimate, declared in zip(profiles, estimates, expected)
    ]
    validate_summary(path, report, totals)


def expected_profiles(
    profiles: Sequence[Mapping[str, Json]],
) -> list[tuple[int, int, int]]:
    """Derive the fixed five geometries at both admitted precisions."""
    base = integer(profiles[0].get("axial_panels"))
    if base < 2 or base > 256 or base % 2:
        raise ValueError("base panel count must be even and within [2,256]")
    geometry = (
        (base, base),
        (2 * base, 2 * base),
        (4 * base, 4 * base),
        (2 * base, 4 * base),
        (4 * base, 2 * base),
    )
    return [
        (precision, axial, radial)
        for precision in (80, 120)
        for axial, radial in geometry
    ]


def validate_profile(
    path: Path,
    profile: Mapping[str, Json],
    estimate: Mapping[str, Json],
    time: str,
    declared: tuple[int, int, int],
) -> tuple[int, int, int]:
    """Check one profile and return its evaluation, root and storage totals."""
    precision, axial, radial = declared
    actual = tuple(
        integer(profile.get(key))
        for key in ("precision", "axial_panels", "radial_squared_panels")
    )
    estimate_profile = tuple(
        integer(estimate.get(key))
        for key in ("precision", "axial_panels", "radial_squared_panels")
    )
    if actual != declared or estimate_profile != actual:
        raise ValueError(f"{path}: profile geometry or precision mismatch")
    if (
        string_value(profile.get("case")) != "similarity-mms-v2"
        or string_value(profile.get("time")) != time
    ):
        raise ValueError(f"{path}: mixed case or clock")
    if any(profile.get(key) != value for key, value in PROFILE_FIELDS.items()):
        raise ValueError(f"{path}: profile convention mismatch")
    evaluations = (axial + 1) * (radial + 1)
    maximum_roots = (axial + 1) * 2048
    reserved = 32 * 1024**2 + precision * 65536
    actual_work = tuple(
        integer(profile.get(key))
        for key in (
            "collar_pressure_evaluations",
            "root_solves",
            "maximum_root_iterations",
            "reserved_bytes",
        )
    )
    if actual_work != (evaluations, axial + 1, maximum_roots, reserved):
        raise ValueError(f"{path}: profile work or storage mismatch")
    if integer(profile.get("cap_bytes")) < reserved:
        raise ValueError(f"{path}: profile cap below reservation")
    return evaluations, maximum_roots, reserved


def validate_summary(
    path: Path, report: dict[str, Json], totals: list[tuple[int, int, int]]
) -> None:
    """Check aggregate counts against the ten individually validated profiles."""
    summary = (
        integer(report.get("sequential_profiles")),
        integer(report.get("accepted_pde_windows")),
        integer(report.get("pressure_evaluations")),
        integer(report.get("maximum_root_iterations")),
        integer(report.get("peak_reserved_bytes")),
        string_value(report.get("classification")),
    )
    expected_summary = (
        10,
        0,
        sum(item[0] for item in totals),
        sum(item[1] for item in totals),
        max(item[2] for item in totals),
        "EmpiricalQuadrature",
    )
    if summary != expected_summary:
        raise ValueError(f"{path}: summary profile mismatch")


def emit_estimates(index: int, report: dict[str, Json]) -> list[str]:
    """Retain every raw mean and its actual profile settings."""
    lines = [f"const ESTIMATES_{index}: [GaugeEstimate; 10] = ["]
    for item in array_value(report["estimates"]):
        row = object_value(item)
        lines.append(
            "    GaugeEstimate { axial_panels: %d, radial_panels: %d, precision: %d, raw_mean: %s },"
            % (
                integer(row.get("axial_panels")),
                integer(row.get("radial_squared_panels")),
                integer(row.get("precision")),
                rust_string(string_value(row.get("mean"))),
            )
        )
    lines.append("];")
    return lines


def emit_changes(index: int, report: dict[str, Json]) -> str:
    """Keep arithmetic and both quadrature directions separate."""
    changes = object_value(report["changes"])
    precision = ", ".join(
        rust_string(string_value(value))
        for value in array_value(changes.get("precision_differences"))
    )
    return f"""const CHANGES_{index}: GaugeChanges = GaugeChanges {{
    coarse_to_middle: {rust_string(string_value(changes.get('coarse_to_middle')))},
    middle_to_fine: {rust_string(string_value(changes.get('middle_to_fine')))},
    axial_only_to_fine: {rust_string(string_value(changes.get('axial_only_to_fine')))},
    radial_only_to_fine: {rust_string(string_value(changes.get('radial_only_to_fine')))},
    precision_differences: [{precision}],
    maximum_precision_to_finest_quadrature_ratio: {option(optional_string(changes.get('maximum_precision_to_finest_quadrature_ratio')))},
    finest_mean: {rust_string(string_value(changes.get('finest_mean')))},
    relative_finest_quadrature_change: {option(optional_string(changes.get('relative_finest_quadrature_change')))},
}};"""


def emit_one(index: int, path: Path) -> list[str]:
    """Emit one hash-bound exact-clock artifact projection."""
    raw, report = read_report(path)
    profiles = array_value(report["profiles"])
    first = object_value(profiles[0])
    elapsed = TIME_TO_TICKS[string_value(first.get("time"))]
    digest = ", ".join(str(byte) for byte in hashlib.sha256(raw).digest())
    lines = [f"const SHA_{index}: [u8; 32] = [{digest}];"]
    lines.extend(emit_estimates(index, report))
    lines.append(emit_changes(index, report))
    lines.append(
        f"const DATA_{index}: GaugeData = GaugeData {{ elapsed: {elapsed}, sha256: SHA_{index}, estimates: &ESTIMATES_{index}, changes: CHANGES_{index} }};"
    )
    return lines


def emit(paths: list[Path]) -> str:
    """Emit deterministic metadata for the complete fixed clock sequence."""
    lines = [
        "// @generated by reference/generate_pressure_gauge_rust.py; raw JSON remains authoritative."
    ]
    for index, path in enumerate(paths):
        lines.extend(emit_one(index, path))
    data = ", ".join(f"DATA_{index}" for index in range(len(paths)))
    lines.append(f"const DATA: [GaugeData; {len(paths)}] = [{data}];")
    return "\n".join(lines) + "\n"


def main() -> None:
    """Write generated source only after every artifact validates."""
    parser = argparse.ArgumentParser()
    parser.add_argument("artifacts", nargs="+", type=Path)
    parser.add_argument("--output", required=True, type=Path)
    args = parser.parse_args()
    output = cast(Path, args.output)
    paths = cast(list[Path], args.artifacts)
    output.write_text(emit(paths), encoding="utf-8")


if __name__ == "__main__":
    main()
