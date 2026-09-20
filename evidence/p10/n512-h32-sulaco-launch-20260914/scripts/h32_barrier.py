"""Barrier handshake primitives shared with the reviewed Rust producer gate.

The solver authenticates a release/abort token by re-deriving
``SHA256(secret || action || '\\n' || nonce || '\\n' || clock || '\\n'
|| state_sha256 || '\\n' || secret)``; this module mirrors that derivation
byte-for-byte so neither side can be fooled by a stray or forged file.  No
raw secret or token value is ever persisted in a receipt.
"""

from __future__ import annotations

import hashlib
import json
import re
import secrets
import sys
import time
from pathlib import Path
from typing import Optional

sys.path.insert(0, str(Path(__file__).resolve().parent))
from h32_types import Env, LogFn, Plan  # noqa: E402

sys.path.insert(0, str(Path(__file__).resolve().parent))
import h32_contract as contract  # noqa: E402
import h32_json as hjson  # noqa: E402
import h32_receipts as receipts  # noqa: E402

POLL_SECONDS = contract.PINNED_WATCHDOG_POLL_SECONDS

ARMED_RECEIPT = "barrier-armed.json"
RELEASE_FILE = "release.token"
ABORT_FILE = "abort.token"
TOKEN_FILES = (RELEASE_FILE, ABORT_FILE)
ARMED_SCHEMA = "p10-avx-n512-temporal-barrier-armed-v1"
MAX_RECEIPT_BYTES = 65536
_HEX64 = re.compile(r"[0-9a-f]{64}")


def existing_tokens(barrier_dir: Path) -> list[str]:
    """Any release/abort token already on disk (foreign or already issued)."""
    return sorted(name for name in TOKEN_FILES if (Path(barrier_dir) / name).exists())


def generate_handshake() -> tuple[str, str]:
    """Fresh 256-bit nonce and secret, returned as lowercase hex."""
    return secrets.token_hex(32), secrets.token_hex(32)


def derive_token(secret_hex: str, action: str, nonce_hex: str, clock: int, state_sha: str) -> str:
    for label, value in (("secret", secret_hex), ("nonce", nonce_hex), ("state", state_sha)):
        if not _HEX64.fullmatch(value):
            raise ValueError(f"{label}_must_be_lowercase_hex64")
    if action not in ("release", "abort"):
        raise ValueError("action_must_be_release_or_abort")
    if type(clock) is not int or clock <= 0:
        raise ValueError("clock_must_be_positive_integer")
    message = (
        f"{action}\n{nonce_hex}\n{clock}\n{state_sha}\n".encode("ascii")
    )
    secret = bytes.fromhex(secret_hex)
    return hashlib.sha256(secret + message + secret).hexdigest()


def barrier_environment(base: Env, stage_barrier_dir: Path, nonce_hex: str,
                        secret_hex: str, clock: int,
                        deadline_epoch: int) -> dict[str, str]:
    env = dict(base)
    env["NSBU_N512_TEMPORAL_BARRIER_DIR"] = str(stage_barrier_dir)
    env["NSBU_N512_TEMPORAL_BARRIER_NONCE"] = nonce_hex
    env["NSBU_N512_TEMPORAL_BARRIER_SECRET"] = secret_hex
    env["NSBU_N512_TEMPORAL_BARRIER_CLOCK"] = str(clock)
    env["NSBU_N512_TEMPORAL_BARRIER_DEADLINE"] = str(deadline_epoch)
    return env


def read_armed_receipt(path: Path, plan: Plan, nonce_hex: str, secret_hex: str,
                       expected_state_sha: Optional[str] = None,
                       ) -> tuple[bool, str, dict[str, object]]:
    """Authenticate the producer's durable armed handshake (reads only)."""
    try:
        raw = path.read_bytes()
    except OSError as error:
        return False, f"armed_receipt_unreadable({error})", {}
    if len(raw) > MAX_RECEIPT_BYTES:
        return False, "armed_receipt_oversized", {}
    try:
        armed = hjson.load_json_object(raw.decode("utf-8"))
    except (UnicodeDecodeError, json.JSONDecodeError) as error:
        return False, f"armed_receipt_not_json({error})", {}
    except hjson.JsonNotObject:
        return False, "armed_receipt_not_object", {}
    if armed.get("schema") != ARMED_SCHEMA:
        return False, "armed_receipt_schema_mismatch", armed
    if armed.get("attempt") != 1 or armed.get("clock") != int(plan["guard"]["first_step_clock"]):
        return False, "armed_receipt_attempt_or_clock_mismatch", armed
    if armed.get("nonce") != nonce_hex:
        return False, "armed_receipt_nonce_mismatch", armed
    if armed.get("qualification") is not False:
        return False, "armed_receipt_qualification_not_false", armed
    state = armed.get("state_sha256")
    if not isinstance(state, str) or not _HEX64.fullmatch(state):
        return False, "armed_receipt_state_malformed", armed
    if expected_state_sha is not None and state != expected_state_sha:
        return False, "armed_receipt_state_mismatch", armed
    deadline = armed.get("deadline_epoch")
    if type(deadline) is not int or deadline <= 0:
        return False, "armed_receipt_deadline_malformed", armed
    return True, "armed_receipt_pass", armed


def issue_token(barrier_dir: Path, action: str, secret_hex: str, nonce_hex: str,
                clock: int, state_sha: str) -> tuple[str, str]:
    """Create-only issue of the authenticated action token.

    Returns (status, reason).  ``written`` proves this process created the
    file; ``uncertain_exists`` means the file pre-existed or raced, so it was
    NOT overwritten and authorship of its bytes is unknown.
    """
    name = RELEASE_FILE if action == "release" else ABORT_FILE
    token = derive_token(secret_hex, action, nonce_hex, clock, state_sha)
    status = receipts.create_bytes(Path(barrier_dir) / name,
                                   (token + "\n").encode("ascii"), 0o600)
    if status == receipts.UNCERTAIN:
        return receipts.UNCERTAIN, f"token_preexists({name})"
    if status != receipts.WRITTEN:
        return "error", f"token_write_failed({status})"
    return receipts.WRITTEN, f"token_issued({action})"


def write_json_receipt(path: Path, payload: dict[str, object]) -> str:
    """Create-only JSON receipt; never overwrites.  Returns a status string.

    ``uncertain_exists`` is deliberately non-raising: the supervisor keeps
    running and reports the uncertainty, because refusing to overwrite is the
    only way to preserve a possibly-valid earlier receipt's bytes.
    """
    return receipts.create_json(path, payload)


def armed_stdout_lines(stdout_text: str) -> list[dict[str, str]]:
    """Every ``temporal_barrier armed`` line as key=value fields, in order."""
    events: list[dict[str, str]] = []
    for line in stdout_text.splitlines():
        if not line.startswith("temporal_barrier armed "):
            continue
        fields: dict[str, str] = {}
        for token in line.split()[2:]:
            if "=" in token:
                key, value = token.split("=", 1)
                fields.setdefault(key, value)
        events.append(fields)
    return events


def one_shot_violation(stdout_text: str, armed_clock: int) -> Optional[str]:
    """The Rust barrier is one-shot: arming twice or arming at any other
    clock after release is a protocol violation the supervisor must abort on."""
    events = armed_stdout_lines(stdout_text)
    if len(events) > 1:
        return f"barrier_rearmed_events({len(events)})"
    if any(event.get("clock") != str(armed_clock) for event in events):
        return f"barrier_armed_wrong_clock({[e.get('clock') for e in events]})"
    return None


def finalise_release_decision(release: bool, doubts: list[str], now_epoch: float,
                              release_deadline_epoch: float) -> tuple[bool, list[str]]:
    """Pure final gate BEFORE any release token: outstanding doubts or a
    crossed FIRST-STEP/RELEASE deadline (a single absolute deadline enforced
    after every wait, validation and receipt) hard-abort the release."""
    if not release:
        return False, []
    reasons = list(doubts)
    if now_epoch >= release_deadline_epoch:
        reasons.append(f"release_deadline_passed(now={now_epoch},"
                       f"deadline={release_deadline_epoch})")
    return (not reasons), reasons


def parse_armed_line(line: str, plan: Plan,
                     nonce_hex: str) -> tuple[bool, str, Optional[str]]:
    del nonce_hex  # the line never carries the nonce; the receipt binds it
    fields: dict[str, str] = {}
    for token in line.split()[2:]:
        if "=" not in token:
            return False, "armed_line_malformed", None
        key, value = token.split("=", 1)
        if key in fields:
            return False, "armed_line_field_duplicated", None
        fields[key] = value
    if fields.get("attempt") != "1":
        return False, "armed_line_attempt_mismatch", None
    if fields.get("clock") != str(plan["guard"]["first_step_clock"]):
        return False, "armed_line_clock_mismatch", None
    if "nonce" in fields:
        return False, "armed_line_leaks_nonce", None
    state = fields.get("state_sha256", "")
    if len(state) != 64 or any(ch not in "0123456789abcdef" for ch in state):
        return False, "armed_line_state_malformed", None
    deadline = fields.get("deadline", "")
    if not deadline.isdigit():
        return False, "armed_line_deadline_malformed", None
    return True, "armed_line_pass", state


class BarrierFlow:
    """Sequenced one-shot barrier handshake between durable clock32 and attempt 2."""

    def __init__(self, plan: Plan, stage: Path, output: Path, logs: Path,
                 log: LogFn) -> None:
        del output  # the handshake itself only needs stage/logs
        self.plan = plan
        self.stage = stage
        self.logs = logs
        self.log = log
        self.nonce, self.secret = generate_handshake()
        self.dir = Path(stage) / "barrier"
        self.token_issued = False
        self.released = False

    def environment(self, base: Env, now: int) -> dict[str, str]:
        barrier_deadline = now + int(self.plan["guard"]["first_step_wall_seconds"]) \
            + int(self.plan["barrier"]["decision_margin_seconds"])
        barrier_deadline = min(barrier_deadline, int(self.plan["guard"]["absolute_deadline_epoch"]))
        return barrier_environment(base, self.dir, self.nonce, self.secret,
                                   int(self.plan["guard"]["first_step_clock"]),
                                   barrier_deadline)

    def wait_armed(self, stdout_path: Path, state_hash: str,
                   release_deadline: float) -> tuple[bool, str]:
        ok, reason, _ = self._wait_armed_line(stdout_path, release_deadline)
        if not ok:
            return False, reason
        armed_deadline = min(time.time() + self.plan["barrier"]["armed_receipt_seconds"],
                             release_deadline)
        while time.time() < armed_deadline:
            ok, reason, _armed = read_armed_receipt(
                self.dir / ARMED_RECEIPT, self.plan, self.nonce, self.secret, state_hash)
            if ok:
                return True, "armed"
            if "unreadable" not in reason:
                return False, reason
            time.sleep(POLL_SECONDS)
        return False, "armed_receipt_timeout"

    def _wait_armed_line(self, stdout_path: Path,
                         release_deadline: float) -> tuple[bool, str, Optional[str]]:
        while time.time() < release_deadline:
            try:
                text = Path(stdout_path).read_text(errors="replace")
            except FileNotFoundError:
                text = ""
            for line in text.splitlines():
                if line.startswith("temporal_barrier armed "):
                    return self.parse_armed_line(line)
            time.sleep(POLL_SECONDS)
        return False, "armed_stdout_line_timeout", None

    def parse_armed_line(self, line: str) -> tuple[bool, str, Optional[str]]:
        return parse_armed_line(line, self.plan, self.nonce)

    def decide_and_issue(self, decision: dict[str, object], state_hash: str,
                         release_deadline: Optional[float] = None) -> tuple[str, str]:
        """Issue the one and only token; never a second one of any kind and
        never a RELEASE past the single absolute first-step/release deadline."""
        if self.token_issued:
            return "refused_protocol_violation", "token_already_issued_this_attempt"
        action = "release" if bool(decision["release"]) else "abort"
        if action == "release" and release_deadline is not None \
                and time.time() >= release_deadline:
            return "refused_deadline", "release_deadline_passed_at_token_issue"
        status, reason = issue_token(self.dir, action, self.secret, self.nonce,
                                     int(self.plan["guard"]["first_step_clock"]),
                                     state_hash)
        if status == receipts.WRITTEN:
            self.token_issued = True
            self.released = action == "release"
        return status, reason
