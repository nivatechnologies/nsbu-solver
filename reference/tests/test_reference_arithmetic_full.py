"""Fast guards for the streamed current-grid arithmetic study."""
import hashlib
import io
import json
from dataclasses import replace
from mpmath import mp, mpf
from pathlib import Path
from collections.abc import Iterator
import unittest
from unittest.mock import patch
from reference.reference_arithmetic_full import (
    Emitter, OUTPUT_CAP, Reducer, bits, evaluate, expected_key, parse_full, preflight, run,
    stop_process, validate_row, validate_vectors,
)
from reference.reference_arithmetic_pilot import category
from tools.json_types import JsonObject


def line(index: tuple[int, int, int], elapsed: int, input_class: str,
         value: str = "0000000000000000", prefix: str = "ARITH_FULL",
         label: str = "full") -> str:
    """Build one correctly shaped synthetic producer row."""
    argument = tuple(bits(item/12.0) for item in index)
    centered = tuple(bits(item/12.0-1.0 if item >= 6 else item/12.0) for item in index)
    time = bits(float(elapsed)*2.0**-20)
    return "\t".join((prefix, label, ",".join(map(str, index)), str(elapsed), input_class,
                      ",".join(argument), ",".join(centered), time, ",".join([value]*42)))+"\n"


class FullRows:
    """Generate the complete fixed producer stream without retaining it."""
    def __iter__(self) -> Iterator[str]:
        for ordinal in range(5184):
            index, elapsed = expected_key(ordinal)
            yield line(index, elapsed, category(index, elapsed))


class FakeProcess:
    """Minimal successful producer used by the streamed orchestration test."""
    def __init__(self) -> None:
        self.stdout = FullRows()
        self.stopped = False

    def wait(self, timeout: float | None = None) -> int:
        """Return successful producer status."""
        return 0

    def poll(self) -> int | None:
        """Report a completed successful producer."""
        return 0

    def terminate(self) -> None:
        """Record a termination request."""
        self.stopped = True

    def kill(self) -> None:
        """Record a kill request."""
        self.stopped = True


class RunningBadProcess(FakeProcess):
    """Malformed live producer used to verify bounded failure cleanup."""
    def __init__(self) -> None:
        super().__init__()
        self.stdout = iter((line((0, 0, 1), 0, "startup"),))

    def poll(self) -> int | None:
        """Remain live until the driver terminates this producer."""
        return -15 if self.stopped else None


class ReferenceArithmeticFull(unittest.TestCase):
    """Exercise ordering, word, finiteness, reducer, and output-cap guards."""

    def test_exact_order_words_classes_and_duplicates_are_enforced(self) -> None:
        seen: set[tuple[tuple[int, int, int], int]] = set()
        first = parse_full(line((0, 0, 0), 0, "startup"))
        validate_row(first, 0, seen)
        self.assertEqual(expected_key(1728), ((0, 0, 0), 64))
        with self.assertRaises(ValueError):
            validate_row(first, 0, seen)
        corrupted = parse_full(line((0, 0, 1), 0, "startup"))
        with self.assertRaises(ValueError):
            validate_row(corrupted, 0, set())

    def test_corrupt_words_classes_and_nonfinite_outputs_are_refused(self) -> None:
        row = line((6, 0, 0), 64, "exterior")
        with self.assertRaises(ArithmeticError):
            validate_row(parse_full(row.replace("3fe0000000000000", "0000000000000000", 1)),
                         1728+6*144, set())
        with self.assertRaises(ArithmeticError):
            validate_row(parse_full(row.replace("exterior", "active")), 1728+6*144, set())
        with self.assertRaises(ArithmeticError):
            validate_row(parse_full(line((0, 0, 0), 0, "startup", "7ff0000000000000")),
                         0, set())
        with self.assertRaises(ArithmeticError):
            validate_row(parse_full(line((6, 0, 0), 64, "exterior", "0000000000000001")),
                         1728+6*144, set())
        valid = parse_full(line((0, 0, 0), 0, "startup"))
        with self.assertRaises(ArithmeticError):
            validate_row(replace(valid, time_bits="0000000000000001"), 0, set())
        zero = evaluate(valid)
        self.assertEqual(zero[-1], 0)

    def test_multiprecision_vector_shape_and_finiteness_are_enforced(self) -> None:
        zero = (mp.mpf(0),)*42
        validate_vectors((zero, zero, zero, zero))
        with self.assertRaises(ArithmeticError):
            validate_vectors((zero[:-1], zero, zero, zero))
        nonfinite = zero[:-1]+(mp.nan,)
        with self.assertRaises(ArithmeticError):
            validate_vectors((zero, zero, nonfinite, zero))

    def test_cleanup_is_bounded_and_skips_completed_process(self) -> None:
        process = FakeProcess()
        stop_process(process)  # type: ignore[arg-type]
        self.assertFalse(process.stopped)
        running = RunningBadProcess()
        with patch("reference.reference_arithmetic_full.subprocess.Popen",
                   return_value=running), self.assertRaises(ValueError):
            run(Path(__file__), io.StringIO())
        self.assertTrue(running.stopped)

    def test_reducer_and_output_cap_retain_bounded_evidence(self) -> None:
        reducer = Reducer("test", "velocity")
        key: JsonObject = {"index": [0, 0, 0], "elapsed_ticks": 64, "class": "active"}
        with mp.workdps(80):
            reducer.add(key, 0, mp.mpf(2), mp.mpf(1))
            reducer.add(key, 1, mp.mpf(1), mp.mpf(1))
            report = reducer.report()
        self.assertEqual(report["maximum_scaled"], "1.0")
        stream = io.StringIO()
        output = Emitter(stream)
        output.emit({"event": "small"})
        self.assertIn("small", stream.getvalue())
        output.bytes = OUTPUT_CAP
        with self.assertRaises(MemoryError):
            output.emit({"event": "too-large"})
        with self.assertRaises(ValueError):
            preflight(Path("missing-reference-arithmetic-binary"), Emitter(io.StringIO()))

    def test_stream_hash_is_sensitive_to_canonical_rows(self) -> None:
        first = hashlib.sha256(line((0, 0, 0), 0, "startup").encode()).hexdigest()
        second = hashlib.sha256(line((0, 0, 1), 0, "startup").encode()).hexdigest()
        self.assertNotEqual(first, second)
        with self.assertRaises(ValueError):
            parse_full("not-a-full-row")

    def test_complete_stream_is_reduced_without_retaining_point_vectors(self) -> None:
        zero = (mp.mpf(0),)*42
        stream = io.StringIO()
        def evaluated_at_required_precision(_row: object) -> tuple[tuple[mpf, ...],
                                                                   tuple[mpf, ...],
                                                                   tuple[mpf, ...],
                                                                   tuple[mpf, ...], int]:
            self.assertEqual(mp.dps, 120)
            return zero, zero, zero, zero, 0
        with patch("reference.reference_arithmetic_full.subprocess.Popen",
                   return_value=FakeProcess()), patch(
                       "reference.reference_arithmetic_full.evaluate",
                       side_effect=evaluated_at_required_precision):
            run(Path(__file__), stream)
        records = [json.loads(item) for item in stream.getvalue().splitlines()]
        self.assertEqual(records[-1]["terminal_row"], 5184)
        self.assertEqual(records[-1]["active_rows"], 1030)
        self.assertTrue(records[-1]["precision_diagnostic"]["passed"])
        self.assertEqual(records[-1]["status"], "completed_guarded_study")
        self.assertLess(len(stream.getvalue().encode()), OUTPUT_CAP)


if __name__ == "__main__":
    unittest.main()
