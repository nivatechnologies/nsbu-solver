"""Strictly atomic create-only receipts shared by every writer in the packet.

Rules (each covered by test_h32_receipts.py):
* every receipt/token/armed-adjacent file is created with O_CREAT|O_EXCL;
  an existing path yields ``uncertain_exists`` and the bytes are NEVER touched;
* the payload is written to the exclusive descriptor, fsynced, and the
  containing directory is fsynced afterwards, so a "written" status proves
  both the bytes and the directory entry are durable;
* the per-attempt ledger remembers every non-``written`` status: the
  supervisor refuses to issue a release token or a success receipt while any
  earlier write in this attempt ended uncertain (an uncertain file could be a
  half-written or foreign artifact the run must not vouch for).
"""

from __future__ import annotations

import json
import os
from pathlib import Path
from typing import Optional

WRITTEN = "written"
UNCERTAIN = "uncertain_exists"


def fsync_dir(path: Path) -> None:
    flags = os.O_RDONLY | getattr(os, "O_DIRECTORY", 0)
    fd = os.open(str(Path(path)), flags)
    try:
        os.fsync(fd)
    finally:
        os.close(fd)


def create_bytes(path: Path, payload: bytes, mode: int = 0o644) -> str:
    """Create-only durable write.  Returns WRITTEN, UNCERTAIN or error(...)."""
    path = Path(path)
    try:
        fd = os.open(str(path), os.O_WRONLY | os.O_CREAT | os.O_EXCL, mode)
    except FileExistsError:
        return UNCERTAIN
    except OSError as error:  # pragma: no cover - device-level failure path
        return f"error({error})"
    try:
        with os.fdopen(fd, "wb") as stream:
            stream.write(payload)
            stream.flush()
            os.fsync(stream.fileno())
    except OSError as error:  # pragma: no cover - disk-full evidence path
        return f"error({error})"
    try:
        fsync_dir(path.parent)
    except OSError as error:  # pragma: no cover - non-syncable fs path
        return f"error(dir_fsync:{error})"
    return WRITTEN


def create_json(path: Path, payload: dict[str, object], mode: int = 0o644) -> str:
    text = json.dumps(payload, indent=2, sort_keys=True) + "\n"
    return create_bytes(path, text.encode("utf-8"), mode)


class ReceiptLedger:
    """One attempt's write outcomes; any non-written entry blocks releases.

    The uncertainty is STICKY at helper level: once a name has been recorded
    with a non-``written`` status, no later ``written`` (or any other) record
    for the same name may overwrite it — an earlier uncertain write could be
    a half-written or foreign artifact this attempt must never vouch for,
    even if a later attempt at the same name succeeded.
    """

    def __init__(self) -> None:
        self.statuses: dict[str, str] = {}

    def record(self, name: str, status: str) -> str:
        previous = self.statuses.get(name)
        if previous is not None and previous != WRITTEN:
            return previous  # sticky: never overwrite an uncertain entry
        self.statuses[name] = status
        return status

    @property
    def uncertain(self) -> list[str]:
        return sorted(name for name, status in self.statuses.items()
                      if status != WRITTEN)

    def refuse_reason(self) -> Optional[str]:
        pending = self.uncertain
        if pending:
            return "receipt_uncertainty_unresolved(" + ",".join(pending) + ")"
        return None
