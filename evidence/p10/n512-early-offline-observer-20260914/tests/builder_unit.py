#!/usr/bin/env python3
"""Focused unit tests for the balance manifest builder's binding and publication.

Covers the Astra-found defects directly: (1) hash and parse must bind to one
bounded byte buffer, (2) publication must be create-only atomic and reject
aliases (existing paths, symlinks, input aliases) and replays, (3) a
post-link(2) directory fsync/close or temporary-cleanup failure must report
explicit publication uncertainty with exit status 2 while preserving the
published destination (injected-failure tests).
"""

from __future__ import annotations

import hashlib
import importlib.util
import json
import os
import sys
import tempfile
from pathlib import Path

SPEC = importlib.util.spec_from_file_location(
    "builder", Path(__file__).resolve().parent.parent / "builder" / "build_balance_manifest.py"
)
builder = importlib.util.module_from_spec(SPEC)
SPEC.loader.exec_module(builder)

passed = 0
failed = 0


def check(name: str, body) -> None:
    global passed, failed
    try:
        body()
        print(f"PASS {name}")
        passed += 1
    except AssertionError as error:
        print(f"FAIL {name}: {error}")
        failed += 1


def refused(body, pattern: str) -> None:
    try:
        body()
    except SystemExit as error:
        assert pattern in str(error), f"expected {pattern!r} in {error!r}"
        return
    raise AssertionError(f"expected refusal containing {pattern!r}")


def temp_dir():
    return tempfile.TemporaryDirectory()


def write(path: Path, data: bytes) -> None:
    path.write_bytes(data)


def no_temp_debris(directory: Path) -> None:
    debris = [name for name in os.listdir(directory) if ".tmp-" in name]
    assert not debris, f"temporary debris left behind: {debris}"


def test_bounded_buffer_cap() -> None:
    with temp_dir() as directory:
        root = Path(directory)
        oversized = root / "oversized.json"
        write(oversized, b"x" * (builder.MAX_INPUT_BYTES + 1))
        refused(lambda: builder.read_bounded(oversized, "input"), "bounded-buffer cap")
        exact = root / "exact.json"
        write(exact, b"x" * builder.MAX_INPUT_BYTES)
        assert len(builder.read_bounded(exact, "input")) == builder.MAX_INPUT_BYTES


def test_hash_parse_same_bytes() -> None:
    with temp_dir() as directory:
        root = Path(directory)
        good = root / "good.json"
        payload = json.dumps({"a": 1}).encode()
        write(good, payload)
        digest = hashlib.sha256(payload).hexdigest()
        assert builder.bind_hash_and_parse(good, "good", digest, "mismatch") == {"a": 1}
        tampered = root / "tampered.json"
        write(tampered, json.dumps({"a": 2}).encode())
        refused(
            lambda: builder.bind_hash_and_parse(tampered, "tampered", digest, "mismatch"),
            "mismatch",
        )
        invalid = root / "invalid.json"
        write(invalid, b"{not json")
        refused(
            lambda: builder.bind_hash_and_parse(
                invalid, "invalid", hashlib.sha256(b"{not json").hexdigest(), "mismatch"
            ),
            "not valid JSON",
        )


def test_publish_new_rejects_replay_and_aliases() -> None:
    with temp_dir() as directory:
        root = Path(directory)
        record = root / "record.json"
        plan = root / "plan.json"
        write(record, b"record-bytes")
        write(plan, b"plan-bytes")
        output = root / "out.json"
        builder.publish_new(output, "manifest\n", (record, plan))
        assert output.read_text() == "manifest\n"
        no_temp_debris(root)
        refused(
            lambda: builder.publish_new(output, "second\n", (record, plan)),
            "already exists",
        )
        assert output.read_text() == "manifest\n", "replay mutated the published bytes"
        alias = root / "alias-to-input"
        os.symlink(plan, alias)
        refused(
            lambda: builder.publish_new(alias, "x\n", (record, plan)),
            "aliases a builder input",
        )
        broken = root / "broken-link"
        os.symlink(root / "nowhere", broken)
        refused(
            lambda: builder.publish_new(broken, "x\n", (record, plan)),
            "already exists",
        )
        hard = root / "hard-to-record"
        os.link(record, hard)
        refused(
            lambda: builder.publish_new(hard, "x\n", (record, plan)),
            "already exists",
        )
        refused(
            lambda: builder.publish_new(plan, "x\n", (record, plan)),
            "aliases a builder input",
        )
        refused(
            lambda: builder.publish_new(root / "mutual.json", "x\n", (record, record)),
            "alias each other",
        )
        no_temp_debris(root)
        assert record.read_bytes() == b"record-bytes"
        assert plan.read_bytes() == b"plan-bytes"


def test_publish_new_is_create_only_under_race() -> None:
    with temp_dir() as directory:
        root = Path(directory)
        record = root / "record.json"
        plan = root / "plan.json"
        write(record, b"r")
        write(plan, b"p")
        output = root / "race.json"
        # A concurrent winner's file appearing after the existence check must
        # still lose atomically at link(2), never overwrite.
        descriptor, temporary = tempfile.mkstemp(dir=root, prefix=".winner.", suffix=".tmp-0")
        with os.fdopen(descriptor, "w") as handle:
            handle.write("winner\n")
        os.link(temporary, output)
        os.unlink(temporary)
        refused(
            lambda: builder.publish_new(output, "loser\n", (record, plan)),
            "already exists",
        )
        assert output.read_text() == "winner\n"
        no_temp_debris(root)


def uncertain(body, output: Path, patterns) -> None:
    import contextlib
    import io

    captured = io.StringIO()
    with contextlib.redirect_stderr(captured):
        try:
            body()
        except SystemExit as error:
            assert error.code == 2, f"expected exit status 2, got {error.code!r}"
        else:
            raise AssertionError("expected exit status 2, no SystemExit raised")
    text = captured.getvalue()
    for pattern in patterns:
        assert pattern in text, f"stderr {text!r} lacks {pattern!r}"
    assert str(output) in text, f"published destination missing from report: {text!r}"


def test_publish_new_uncertainty_directory_fsync() -> None:
    with temp_dir() as directory:
        root = Path(directory)
        record = root / "record.json"
        plan = root / "plan.json"
        write(record, b"r")
        write(plan, b"p")
        output = root / "fsync-uncertain.json"
        opened = []
        original_open = os.open
        original_fsync = os.fsync

        def spy_open(*args, **kwargs):
            descriptor = original_open(*args, **kwargs)
            flags = args[1] if len(args) > 1 else kwargs.get("flags", 0)
            if flags == os.O_RDONLY:  # post-link directory durability fd only
                opened.append(descriptor)
            return descriptor

        def flaky_fsync(descriptor):
            if descriptor in opened:  # only the post-link directory fd
                raise OSError(5, "injected directory fsync failure")
            return original_fsync(descriptor)

        os.open = spy_open
        os.fsync = flaky_fsync
        try:
            uncertain(
                lambda: builder.publish_new(output, "manifest\n", (record, plan)),
                output,
                ("publication uncertain", "directory fsync/close",
                 "injected directory fsync failure", "errno=5"),
            )
        finally:
            os.open = original_open
            os.fsync = original_fsync
        assert output.exists(), "published destination lost"
        assert output.read_text() == "manifest\n", "published bytes mutated"
        no_temp_debris(root)


def test_publish_new_uncertainty_directory_close() -> None:
    with temp_dir() as directory:
        root = Path(directory)
        record = root / "record.json"
        plan = root / "plan.json"
        write(record, b"r")
        write(plan, b"p")
        output = root / "close-uncertain.json"
        opened = []
        original_open = os.open
        original_close = os.close

        def spy_open(*args, **kwargs):
            descriptor = original_open(*args, **kwargs)
            flags = args[1] if len(args) > 1 else kwargs.get("flags", 0)
            if flags == os.O_RDONLY:  # post-link directory durability fd only
                opened.append(descriptor)
            return descriptor

        def flaky_close(descriptor):
            if descriptor in opened:  # only the post-link directory fd
                raise OSError(9, "injected directory close failure")
            return original_close(descriptor)

        os.open = spy_open
        os.close = flaky_close
        try:
            uncertain(
                lambda: builder.publish_new(output, "manifest\n", (record, plan)),
                output,
                ("publication uncertain", "directory fsync/close",
                 "injected directory close failure", "errno=9"),
            )
        finally:
            os.open = original_open
            os.close = original_close
        assert output.exists(), "published destination lost"
        assert output.read_text() == "manifest\n", "published bytes mutated"
        no_temp_debris(root)


def test_publish_new_uncertainty_temporary_cleanup() -> None:
    with temp_dir() as directory:
        root = Path(directory)
        record = root / "record.json"
        plan = root / "plan.json"
        write(record, b"r")
        write(plan, b"p")
        output = root / "cleanup-uncertain.json"
        original_unlink = os.unlink

        def flaky_unlink(path):
            raise OSError(16, "injected temporary cleanup failure")

        os.unlink = flaky_unlink
        try:
            uncertain(
                lambda: builder.publish_new(output, "manifest\n", (record, plan)),
                output,
                ("publication uncertain", "temporary-file cleanup",
                 "injected temporary cleanup failure", "errno=16"),
            )
        finally:
            os.unlink = original_unlink
        assert output.exists(), "published destination lost"
        assert output.read_text() == "manifest\n", "published bytes mutated"
        debris = [name for name in os.listdir(root) if ".tmp-" in name]
        assert debris, "failed cleanup must leave the temporary file behind"


def main() -> None:
    for name, body in sorted(globals().items()):
        if name.startswith("test_"):
            check(name, body)
    print(f"== builder unit tests: {passed} passed, {failed} failed")
    sys.exit(1 if failed else 0)


if __name__ == "__main__":
    main()
