"""Stream the admitted current-grid 80/120-digit reference-arithmetic study."""
from dataclasses import dataclass
import hashlib
import json
import math
from mpmath import mp, mpf
from pathlib import Path
import struct
import subprocess
import sys
from time import perf_counter
from typing import Protocol, TextIO, cast
from reference.reference_arithmetic_pilot import (
    CLOCKS, QUANTITIES, RustRow, category, exact_float, float_from_bits,
    parse_row, point_category, rational_point, tracking_values, word_point,
)
from tools.json_types import JsonObject

CASE = Path("benchmarks/similarity-mms-v2.json")
CASE_SHA256 = "e1236f7b3c51537acd17381402ca420ba7872a7b9dbc64b2f0d9d5108a468f7e"
OUTPUT_CAP = 1024*1024
PROGRESS_INTERVAL = 32
COMPARISONS = ("rational_80_to_120", "word_80_to_120",
               "rational_to_word_at_120", "rust_binary64_to_word_120")


class Digest(Protocol):
    """Minimal streaming SHA-256 interface used by the study."""
    def update(self, value: bytes, /) -> None:
        """Add bytes to the digest."""
        ...

    def hexdigest(self) -> str:
        """Return the current lowercase hexadecimal digest."""
        ...


@dataclass
class Reducer:
    """Streaming arithmetic-error summary for one tracking quantity."""
    comparison: str
    quantity: str
    count: int = 0
    squares: mpf = mpf(0)
    maximum_absolute: mpf = mpf(0)
    maximum_scaled: mpf = mpf(0)
    absolute_witness: JsonObject | None = None
    scaled_witness: JsonObject | None = None

    def add(self, key: JsonObject, component: int, left: mpf, right: mpf) -> None:
        """Accumulate one component without retaining its full-grid value."""
        absolute = abs(left-right)
        scaled = absolute/max(mp.mpf(1), abs(right))
        self.count += 1
        self.squares += scaled*scaled
        if absolute > self.maximum_absolute:
            self.maximum_absolute = absolute
            self.absolute_witness = witness(key, component, left, right)
        if scaled > self.maximum_scaled:
            self.maximum_scaled = scaled
            self.scaled_witness = witness(key, component, left, right)

    def report(self) -> JsonObject:
        """Return compact maxima, RMS, and witnesses."""
        return {"comparison": self.comparison, "quantity": self.quantity, "components": self.count,
                "maximum_absolute": mp.nstr(self.maximum_absolute, 30),
                "maximum_scaled": mp.nstr(self.maximum_scaled, 30),
                "rms_scaled": mp.nstr(mp.sqrt(self.squares/self.count), 30),
                "absolute_witness": self.absolute_witness,
                "scaled_witness": self.scaled_witness}


class Emitter:
    """Flush bounded JSON Lines so timeout evidence remains readable."""
    def __init__(self, stream: TextIO) -> None:
        self.stream = stream
        self.bytes = 0

    def emit(self, record: JsonObject) -> None:
        """Write one record only while the complete stream remains under 1 MiB."""
        encoded = json.dumps(record, separators=(",", ":"))+"\n"
        self.bytes += len(encoded.encode())
        if self.bytes > OUTPUT_CAP:
            raise MemoryError("Reference arithmetic summary exceeded 1 MiB")
        self.stream.write(encoded)
        self.stream.flush()


def witness(key: JsonObject, component: int, left: mpf, right: mpf) -> JsonObject:
    """Retain the exact location and bounded decimal values for one extremum."""
    return {**key, "component": component, "left": mp.nstr(left, 40),
            "right": mp.nstr(right, 40)}


def bits(value: float) -> str:
    """Encode one Python binary64 value in the producer's canonical form."""
    return f"{struct.unpack('>Q', struct.pack('>d', value))[0]:016x}"


def expected_key(ordinal: int) -> tuple[tuple[int, int, int], int]:
    """Return the required z-fast row key for one zero-based ordinal."""
    elapsed = CLOCKS[ordinal//1728]
    flat = ordinal % 1728
    return (flat//144, (flat//12) % 12, flat % 12), elapsed


def validate_words(row: RustRow) -> None:
    """Independently validate argument, centering, time, class, and finite outputs."""
    expected_arguments = tuple(bits(value/12.0) for value in row.index)
    arguments = tuple(float_from_bits(word) for word in row.argument_bits)
    expected_centered = tuple(bits(value-1.0 if value >= 0.5 else value)
                              for value in arguments)
    expected_time = bits(float(row.elapsed)*2.0**-20)
    if row.argument_bits != expected_arguments or row.centered_bits != expected_centered:
        raise ArithmeticError("Producer coordinate words do not match binary64 i/12 and centering")
    if row.time_bits != expected_time:
        raise ArithmeticError("Producer time word does not match the exact tick conversion")
    rational_class = category(row.index, row.elapsed)
    word_class = point_category(word_point(row))
    if row.category != rational_class or word_class != rational_class:
        raise ArithmeticError("Producer, rational, and binary-word classes differ")
    if any(not math.isfinite(float_from_bits(word)) for word in row.value_bits):
        raise ArithmeticError("Producer emitted a nonfinite reference component")
    if row.category != "active" and any(int(word, 16) != 0 for word in row.value_bits):
        raise ArithmeticError("Producer emitted a nonzero shortcut component")


def validate_row(row: RustRow, ordinal: int, seen: set[tuple[tuple[int, int, int], int]]) -> None:
    """Require the exact row identity, order, uniqueness, and independently checked words."""
    key = (row.index, row.elapsed)
    if row.label != "full" or key != expected_key(ordinal):
        raise ValueError("Producer row identity or order mismatch")
    if key in seen:
        raise ValueError("Duplicate producer row")
    seen.add(key)
    validate_words(row)


def vector_hash(digest: Digest, row: RustRow, values: tuple[mpf, ...], digits: int) -> None:
    """Update one deterministic high-precision value-stream digest."""
    prefix = f"{row.elapsed}:{row.index[0]},{row.index[1]},{row.index[2]}:"
    digest.update((prefix+",".join(mp.nstr(value, digits) for value in values)+"\n").encode())


def validate_vectors(vectors: tuple[tuple[mpf, ...], ...]) -> None:
    """Reject malformed or nonfinite multiprecision results before reduction."""
    if len(vectors) != 4 or any(len(values) != 42 for values in vectors):
        raise ArithmeticError("Each of the four multiprecision vectors must have 42 components")
    if any(not mp.isfinite(value) for values in vectors for value in values):
        raise ArithmeticError("Multiprecision evaluator emitted a nonfinite component")


def evaluate(row: RustRow) -> tuple[tuple[mpf, ...], tuple[mpf, ...],
                                    tuple[mpf, ...], tuple[mpf, ...], int]:
    """Hold only four precision/binding vectors for one independently classified point."""
    if row.category != "active":
        zero = (mp.mpf(0),)*42
        return zero, zero, zero, zero, 0
    iterations = 0
    with mp.workdps(80):
        rational80, used = tracking_values(rational_point(row))
        iterations = max(iterations, used)
        words80, used = tracking_values(word_point(row))
        iterations = max(iterations, used)
    with mp.workdps(120):
        rational120, used = tracking_values(rational_point(row))
        iterations = max(iterations, used)
        words120, used = tracking_values(word_point(row))
        iterations = max(iterations, used)
    return rational80, words80, rational120, words120, iterations


def reducers() -> dict[tuple[str, str], Reducer]:
    """Construct the fixed comparison-by-quantity reduction table."""
    return {(comparison, quantity): Reducer(comparison, quantity)
            for comparison in COMPARISONS for quantity, _ in QUANTITIES}


def reduce_row(table: dict[tuple[str, str], Reducer], row: RustRow,
               vectors: tuple[tuple[mpf, ...], tuple[mpf, ...],
                              tuple[mpf, ...], tuple[mpf, ...]]) -> None:
    """Reduce four per-point vectors, then allow the caller to discard them."""
    rational80, words80, rational120, words120 = vectors
    rust = tuple(exact_float(word) for word in row.value_bits)
    pairs = ((rational80, rational120), (words80, words120),
             (rational120, words120), (rust, words120))
    key: JsonObject = {"index": list(row.index), "elapsed_ticks": row.elapsed,
                       "class": row.category}
    offset = 0
    for quantity, count in QUANTITIES:
        for comparison, (left, right) in zip(COMPARISONS, pairs, strict=True):
            for component in range(offset, offset+count):
                table[(comparison, quantity)].add(key, component-offset,
                                                   left[component], right[component])
        offset += count


def parse_full(line: str) -> RustRow:
    """Parse a full-producer row through the shared shape checker."""
    if not line.startswith("ARITH_FULL\t"):
        raise ValueError("Not a full reference row")
    return parse_row("ARITH_PILOT\t"+line.removeprefix("ARITH_FULL\t").rstrip("\n"))


def preflight(binary: Path, output: Emitter) -> None:
    """Freeze the case and finite full-study work before starting the producer."""
    case_hash = hashlib.sha256(CASE.read_bytes()).hexdigest()
    if case_hash != CASE_SHA256 or not binary.is_file():
        raise ValueError("Case hash or prebuilt Rust producer mismatch")
    output.emit({"event": "admission", "case_sha256": case_hash, "rows": 5184,
                 "rust_reference_evaluator_calls": 10368,
                 "multiprecision_jet_evaluations": 4120,
                 "maximum_scalar_root_iterations": 8437760,
                 "fixed_jet_corrections": 12360, "output_cap_bytes": OUTPUT_CAP,
                 "progress_interval": PROGRESS_INTERVAL})


def complete_report(table: dict[tuple[str, str], Reducer], hashes: dict[str, Digest],
                    rust_hash: Digest, rows: int, active: int, maximum_root: int,
                    started: float, output: Emitter) -> JsonObject:
    """Build the terminal record only after the producer and all rows succeed."""
    observed = max(table[(comparison, quantity)].maximum_absolute
                   for comparison in COMPARISONS[:2] for quantity, _ in QUANTITIES)
    criterion = mp.mpf("1e-60")
    diagnostic_passed = observed <= criterion
    status = ("completed_guarded_study" if diagnostic_passed else
              "completed_guarded_study_precision_diagnostic_failed")
    return {"event": "complete", "status": status, "terminal_row": rows,
            "active_rows": active, "shortcut_rows": rows-active,
            "multiprecision_jet_evaluations": 4*active,
            "rust_reference_evaluator_calls": 2*rows,
            "maximum_observed_root_iterations": maximum_root,
            "seconds": perf_counter()-started, "rust_rows_sha256": rust_hash.hexdigest(),
            "value_stream_sha256": {name: digest.hexdigest() for name, digest in hashes.items()},
            "precision_diagnostic": {"comparison": "80_to_120_digits",
                                     "maximum_absolute": mp.nstr(observed, 30),
                                     "criterion": "1e-60", "passed": diagnostic_passed,
                                     "role": "retained diagnostic, not a binary64 acceptance gate"},
            "reducers": [item.report() for item in table.values()],
            "output_bytes_before_terminal": output.bytes,
            "limitations": ["empirical reference-evaluator arithmetic, not an enclosure",
                            "no binary64 acceptance threshold", "no integrator arithmetic",
                            "no continuum bound", "raw pressure and gauge excluded",
                            "no PDE trajectory, convergence, or accepted-window claim"]}


def stop_process(process: subprocess.Popen[str]) -> None:
    """Bound cleanup of a producer after parse, arithmetic, or output failure."""
    if process.poll() is not None:
        return
    process.terminate()
    try:
        process.wait(timeout=5)
    except subprocess.TimeoutExpired:
        process.kill()
        process.wait(timeout=5)


def run(binary: Path, stream: TextIO = sys.stdout) -> None:
    """Stream, validate, evaluate, hash, and reduce the admitted full grid."""
    output = Emitter(stream)
    preflight(binary, output)
    command = [str(binary), "--exact", "emit_full_binary64_reference_rows", "--nocapture"]
    process: subprocess.Popen[str] = subprocess.Popen(
        command, stdout=subprocess.PIPE, stderr=None, text=True, bufsize=1)
    if process.stdout is None:
        stop_process(process)
        raise RuntimeError("Rust producer stdout pipe unavailable")
    stdout = cast(TextIO, process.stdout)
    table = reducers()
    hashes: dict[str, Digest] = {name: hashlib.sha256() for name in
                                ("rational80", "word80", "rational120", "word120")}
    rust_hash: Digest = hashlib.sha256()
    seen: set[tuple[tuple[int, int, int], int]] = set()
    rows = active = maximum_root = 0
    started = perf_counter()
    try:
        for line in stdout:
            if not line.startswith("ARITH_FULL\t"):
                continue
            row = parse_full(line)
            with mp.workdps(120):
                validate_row(row, rows, seen)
                rust_hash.update(line.encode())
                rational80, words80, rational120, words120, used = evaluate(row)
                vectors = (rational80, words80, rational120, words120)
                validate_vectors(vectors)
                maximum_root = max(maximum_root, used)
                active += int(row.category == "active")
                for name, values, digits in (("rational80", rational80, 80),
                                             ("word80", words80, 80),
                                             ("rational120", rational120, 120),
                                             ("word120", words120, 120)):
                    vector_hash(hashes[name], row, values, digits)
                reduce_row(table, row, vectors)
            rows += 1
            if rows % PROGRESS_INTERVAL == 0:
                output.emit({"event": "progress", "rows": rows, "active_rows": active,
                             "seconds": perf_counter()-started,
                             "rust_rows_sha256": rust_hash.hexdigest(),
                             "value_stream_sha256": {name: value.hexdigest()
                                                     for name, value in hashes.items()}})
        status = process.wait()
        if status != 0:
            raise RuntimeError(f"Rust producer failed with status {status}")
        if rows != 5184 or len(seen) != rows or expected_key(rows-1) != ((11, 11, 11), 128):
            raise ValueError("Full producer did not complete exactly 5,184 unique ordered rows")
        if active != 1030:
            raise ValueError("Full producer active-row count mismatch")
        with mp.workdps(120):
            report = complete_report(table, hashes, rust_hash, rows, active,
                                     maximum_root, started, output)
            output.emit(report)
    except BaseException:
        stop_process(process)
        raise


def main(argv: list[str]) -> None:
    """Require exactly one already-built fixed Rust producer."""
    if len(argv) != 2:
        raise SystemExit("usage: python -m reference.reference_arithmetic_full RUST_TEST_BINARY")
    run(Path(argv[1]).resolve())


if __name__ == "__main__":
    main(sys.argv)
