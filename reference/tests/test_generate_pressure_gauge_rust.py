"""Deterministic generation and malformed pressure-gauge artifact controls."""

import json
from pathlib import Path
import subprocess
from tempfile import TemporaryDirectory
import pytest
from reference.generate_pressure_gauge_rust import (
    emit,
    emit_one,
    integer,
    optional_string,
)
from tools.json_types import Json, array_value, decode, object_value

ROOT = Path(__file__).resolve().parents[2]
ARTIFACTS = [
    ROOT / "crates/nsbu-benchmarks/data/v2-pressure-gauge" / f"clock{clock}.json"
    for clock in (0, 64, 128)
]
GENERATED = (
    ROOT / "crates/nsbu-benchmarks/src/v2_experiment/pressure_reference/gauge_data.rs"
)


def write_changed(directory: Path, **changes: Json) -> Path:
    """Write one valid artifact with selected top-level replacements."""
    report = dict(object_value(decode(ARTIFACTS[1].read_text())))
    report.update(changes)
    result = directory / "changed.json"
    result.write_text(json.dumps(report))
    return result


def test_complete_artifacts_regenerate_the_frozen_projection() -> None:
    """Formatting deterministic generator output reproduces committed Rust bytes."""
    with TemporaryDirectory() as raw:
        output = Path(raw) / "gauge_data.rs"
        output.write_text(emit(ARTIFACTS))
        subprocess.run(["rustfmt", "--edition", "2021", str(output)], check=True)
        assert output.read_bytes() == GENERATED.read_bytes()


def test_malformed_shape_time_and_scalar_types_refuse() -> None:
    """Incomplete reports and ambiguous integer/string values fail closed."""
    with TemporaryDirectory() as raw:
        directory = Path(raw)
        with pytest.raises(ValueError):
            emit([write_changed(directory, status="incomplete")])
        report = dict(object_value(decode(ARTIFACTS[1].read_text())))
        estimates = array_value(report["estimates"])
        with pytest.raises(ValueError):
            emit([write_changed(directory, estimates=estimates[:-1])])
        profiles = [
            dict(object_value(item)) for item in array_value(report["profiles"])
        ]
        profiles[0]["time"] = "1/4096"
        report["profiles"] = profiles
        changed = directory / "clock.json"
        changed.write_text(json.dumps(report))
        with pytest.raises(KeyError):
            emit_one(0, changed)
    with pytest.raises(ValueError):
        integer(True)
    with pytest.raises(ValueError):
        optional_string(3)
    assert integer(8) == 8
    assert optional_string(None) is None
