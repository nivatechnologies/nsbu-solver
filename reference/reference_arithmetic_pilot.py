"""Bounded 80/120-digit current-grid reference-arithmetic timing pilot."""
from dataclasses import dataclass
import json
from mpmath import mp, mpf
from pathlib import Path
import struct
import subprocess
import sys
from time import perf_counter
from reference.evaluator import field_jets
from reference.jets import Jet
from reference.scalar import rational
from reference.tuples import quadruple
from tools.json_types import JsonObject

POINTS = (("axis", (0, 0, 1)), ("collar", (5, 0, 0)),
          ("interior", (1, 1, 1)), ("exterior", (6, 0, 0)))
CLOCKS = (0, 64, 128)
QUANTITIES = (("velocity", 3), ("gradient", 9), ("ordered_hessian", 27), ("curl", 3))
Results = dict[tuple[str, int, str, int], tuple[mpf, ...]]


@dataclass(frozen=True)
class RustRow:
    """One source-emitted binary64 pilot row."""
    label: str
    index: tuple[int, int, int]
    elapsed: int
    category: str
    argument_bits: tuple[str, str, str]
    centered_bits: tuple[str, str, str]
    time_bits: str
    value_bits: tuple[str, ...]


def centered_numerator(index: int) -> int:
    """Return the exact centered numerator for one index on the 12-grid."""
    return index if index < 6 else index-12


def category(index: tuple[int, int, int], elapsed: int) -> str:
    """Classify the exact case shortcut without binary64 arithmetic."""
    if elapsed == 0:
        return "startup"
    radius_numerator = sum(centered_numerator(value)**2 for value in index)
    return "exterior" if radius_numerator*2500 >= 441*144 else "active"


def full_grid_counts() -> dict[str, int]:
    """Return exact current-grid input-class counts for the three clocks."""
    exterior = sum(category((x, y, z), 64) == "exterior"
                   for x in range(12) for y in range(12) for z in range(12))
    return {"rows": 3*12**3, "exterior_spatial": exterior,
            "exterior_rows": 3*exterior, "startup_rows": 12**3,
            "startup_exterior_intersection": exterior,
            "shortcut_union_rows": 12**3+2*exterior,
            "active_rows": 2*(12**3-exterior)}


def float_from_bits(word: str) -> float:
    """Decode one canonical big-endian binary64 word."""
    return struct.unpack(">d", int(word, 16).to_bytes(8, "big"))[0]


def exact_float(word: str) -> mpf:
    """Convert a finite binary64 word to its exact multiprecision value."""
    numerator, denominator = float_from_bits(word).as_integer_ratio()
    return mp.mpf(numerator)/denominator


def parse_row(line: str) -> RustRow:
    """Parse and shape-check one Rust source row."""
    columns = line.split("\t")
    if len(columns) != 9 or columns[0] != "ARITH_PILOT":
        raise ValueError("Malformed Rust pilot row")
    value_bits = tuple(columns[8].split(","))
    if len(value_bits) != 42:
        raise ValueError("Rust pilot row must contain 42 tracking entries")
    index_parts = columns[2].split(",")
    argument_parts = columns[5].split(",")
    centered_parts = columns[6].split(",")
    if any(len(value) != 3 for value in (index_parts, argument_parts, centered_parts)):
        raise ValueError("Rust pilot coordinate word shape mismatch")
    index = (int(index_parts[0]), int(index_parts[1]), int(index_parts[2]))
    argument = (argument_parts[0], argument_parts[1], argument_parts[2])
    centered = (centered_parts[0], centered_parts[1], centered_parts[2])
    return RustRow(columns[1], index, int(columns[3]), columns[4], argument, centered,
                   columns[7], value_bits)


def rust_rows(binary: Path) -> tuple[list[RustRow], str]:
    """Execute the already-built fixed Rust producer and retain its output digest."""
    completed = subprocess.run([str(binary), "--exact", "emit_fixed_binary64_reference_rows",
                                "--nocapture"], capture_output=True, text=True, check=True)
    lines = [line for line in completed.stdout.splitlines() if line.startswith("ARITH_PILOT\t")]
    rows = [parse_row(line) for line in lines]
    if len(rows) != 12:
        raise ValueError("Rust producer did not emit the fixed 12 rows")
    import hashlib
    return rows, hashlib.sha256(("\n".join(lines)+"\n").encode()).hexdigest()


def rational_point(row: RustRow) -> tuple[mpf, mpf, mpf, mpf]:
    """Construct exact centered 12-grid coordinates and the exact dyadic clock."""
    spatial = tuple(mp.mpf(centered_numerator(value))/12 for value in row.index)
    return quadruple((*spatial, mp.mpf(row.elapsed)/2**20))


def word_point(row: RustRow) -> tuple[mpf, mpf, mpf, mpf]:
    """Construct the exact values of the Rust evaluator's centered input words."""
    return quadruple((*[exact_float(word) for word in row.centered_bits], exact_float(row.time_bits)))


def tracking_values(point: tuple[mpf, mpf, mpf, mpf]) -> tuple[tuple[mpf, ...], int]:
    """Evaluate only the four tracking quantities with the independent Python jets."""
    variables = quadruple(Jet.variable(value, axis) for axis, value in enumerate(point))
    velocity, _, solution = field_jets(*variables)
    gradient = [[value.derivative(axis) for axis in range(3)] for value in velocity]
    values = [value.value for value in velocity]
    values.extend(value.value for row in gradient for value in row)
    values.extend(value.derivative(axis).value
                  for row in gradient for value in row for axis in range(3))
    values.extend((gradient[2][1].value-gradient[1][2].value,
                   gradient[0][2].value-gradient[2][0].value,
                   gradient[1][0].value-gradient[0][1].value))
    if len(values) != 42:
        raise ArithmeticError("Python pilot must produce 42 tracking entries")
    return tuple(values), solution.scalar.iterations


def point_category(point: tuple[mpf, mpf, mpf, mpf]) -> str:
    """Classify an already-bound high-precision input."""
    if point[3] == 0:
        return "startup"
    return "exterior" if sum(value*value for value in point[:3]) >= rational("441/2500") else "active"


def emit(record: JsonObject) -> None:
    """Flush one complete record so a numerical timeout preserves partial work."""
    print(json.dumps(record, separators=(",", ":")), flush=True)


def maximum_scaled(left: tuple[mpf, ...], right: tuple[mpf, ...]) -> list[str]:
    """Report one maximum scaled change for each tracking quantity."""
    result: list[str] = []
    offset = 0
    for _, count in QUANTITIES:
        pairs = zip(left[offset:offset+count], right[offset:offset+count], strict=True)
        value = max(abs(a-b)/max(mp.mpf(1), abs(b)) for a, b in pairs)
        result.append(mp.nstr(value, 30))
        offset += count
    return result


def validate_rows(rows: list[RustRow]) -> dict[str, int]:
    """Require the fixed inputs and all three input classes."""
    expected = {(label, index, elapsed) for label, index in POINTS for elapsed in CLOCKS}
    observed = {(row.label, row.index, row.elapsed) for row in rows}
    if observed != expected:
        raise ValueError("Rust input rows do not match the fixed pilot")
    counts = {name: sum(row.category == name for row in rows)
              for name in ("startup", "exterior", "active")}
    if counts != {"startup": 4, "exterior": 2, "active": 6}:
        raise ValueError("Unexpected Rust input-class counts")
    return counts


def evaluate_one(row: RustRow, binding: str, precision: int) -> tuple[tuple[mpf, ...], int, bool]:
    """Evaluate one bound point, directly validating zero classes at 120 digits."""
    point = rational_point(row) if binding == "rational" else word_point(row)
    if point_category(point) != row.category or category(row.index, row.elapsed) != row.category:
        raise ArithmeticError("Rational, binary-word, and Rust input classes differ")
    direct = row.category == "active" or precision == 120
    values, iterations = tracking_values(point) if direct else ((mp.mpf(0),)*42, 0)
    if row.category != "active" and any(value != 0 for value in values):
        raise ArithmeticError("A validated shortcut row was not exactly zero")
    if row.category != "active" and any(int(word, 16) != 0 for word in row.value_bits):
        raise ArithmeticError("Rust disagrees with an exact zero shortcut")
    return values, iterations, direct


def evaluate_rows(rows: list[RustRow]) -> tuple[Results, int, int]:
    """Evaluate fixed rows sequentially while retaining timeout-visible progress."""
    results: Results = {}
    maximum_root_iterations = 0
    direct_evaluations = 0
    for precision in (80, 120):
        for binding in ("rational", "binary64_words"):
            with mp.workdps(precision):
                for row in rows:
                    begin = perf_counter()
                    values, iterations, direct = evaluate_one(row, binding, precision)
                    direct_evaluations += int(direct)
                    maximum_root_iterations = max(maximum_root_iterations, iterations)
                    results[(row.label, row.elapsed, binding, precision)] = values
                    emit({"event": "evaluation", "sample": row.label, "elapsed_ticks": row.elapsed,
                          "class": row.category, "binding": binding, "precision": precision,
                          "direct": direct, "seconds": perf_counter()-begin,
                          "root_iterations": iterations})
    return results, direct_evaluations, maximum_root_iterations


def compare_rows(rows: list[RustRow], results: Results) -> list[JsonObject]:
    """Compare precision, input rounding, and binary64 arithmetic separately."""
    comparisons: list[JsonObject] = []
    with mp.workdps(120):
        for row in rows:
            rational80 = results[(row.label, row.elapsed, "rational", 80)]
            rational120 = results[(row.label, row.elapsed, "rational", 120)]
            words80 = results[(row.label, row.elapsed, "binary64_words", 80)]
            words120 = results[(row.label, row.elapsed, "binary64_words", 120)]
            comparisons.append({
                "sample": row.label, "elapsed_ticks": row.elapsed, "class": row.category,
                "rational_80_to_120": maximum_scaled(rational80, rational120),
                "word_80_to_120": maximum_scaled(words80, words120),
                "rational_to_word_at_120": maximum_scaled(rational120, words120),
                "rust_binary64_to_word_120": maximum_scaled(
                    tuple(exact_float(word) for word in row.value_bits), words120),
            })
    return comparisons


def run(binary: Path) -> None:
    """Run the fixed pilot and emit JSON Lines progress plus a final comparison."""
    rows, rust_digest = rust_rows(binary)
    counts = validate_rows(rows)
    emit({"event": "admission", "pilot_rows": len(rows), "pilot_classes": counts,
          "full_grid_classes": full_grid_counts(), "rust_rows_sha256": rust_digest})
    started = perf_counter()
    results, direct_evaluations, maximum_root_iterations = evaluate_rows(rows)
    emit({"event": "complete", "status": "passed", "seconds": perf_counter()-started,
          "direct_mp_evaluations": direct_evaluations,
          "maximum_scalar_root_iterations": maximum_root_iterations,
          "quantity_order": [name for name, _ in QUANTITIES],
          "input_classes_match": True, "validated_zero_rows": counts["startup"]+counts["exterior"],
          "comparisons": compare_rows(rows, results),
          "limitations": ["single bounded timing pilot", "no binary64 acceptance threshold",
                          "no integrator arithmetic or continuum bound", "pressure and gauge excluded",
                          "no full-grid execution, PDE trajectory, convergence, or accepted-window claim"]})


def main(argv: list[str]) -> None:
    """Require exactly one prebuilt fixed Rust producer path."""
    if len(argv) != 2:
        raise SystemExit("usage: python -m reference.reference_arithmetic_pilot RUST_TEST_BINARY")
    run(Path(argv[1]).resolve())


if __name__ == "__main__":
    main(sys.argv)
