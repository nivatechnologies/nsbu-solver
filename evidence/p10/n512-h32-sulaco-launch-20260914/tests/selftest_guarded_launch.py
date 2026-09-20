#!/usr/bin/env python3
"""End-to-end guarded-launch selftest with a fake solver and the real watchdog.

Runs the production driver in-process against a temp stage: the fake solver
performs the real authenticated barrier handshake (token derivation included),
writes a structurally exact snapshot (layout derived from artifact.rs) and
cannot begin its simulated attempt 2 until the driver issues a valid release
token.  The fake solver emulates the ONE-SHOT Rust barrier: it arms once at
clock 32, releases, and never arms again.  Cases: positive release, abort
(tampered attempt.json), a re-arm-attempt case proving the supervisor refuses
a solver that would arm twice, and a decision-receipt-uncertain case proving
the supervisor hard-aborts BEFORE RELEASE when the decision receipt is not
durably its own create-only write.  No real solver, N512 field, host binding
or scientific claim is involved.
"""

from __future__ import annotations

import json
import os
import shutil
import sys
import tempfile
import time
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE.parent / "scripts"))
import h32_contract as contract  # noqa: E402
import h32_launch_supervisor as driver  # noqa: E402
import process_guard as guard  # noqa: E402  (VERIFIED subreaper + atexit drain)
from test_h32_bundles import IDENTITY  # noqa: E402
from test_h32_contract import plan_template  # noqa: E402

if not guard.SUBREAPER_ACTIVE:
    # The SELFTEST validates the production ownership model; running it
    # without a verified subreaper would make its zero-leak claim hollow.
    raise SystemExit("refused: PR_SET_CHILD_SUBREAPER unavailable (selftest)")


def sweep_own_process_groups() -> None:
    """UNCONDITIONAL sweep+complete waitpid reaping on EVERY run_case exit
    path (fake solvers park until an absolute 2026 deadline, so a missed
    drain would be a weeks-long orphan).  Kills every remaining child group
    AND reaps every zombie; a surviving descendant raises LOUDLY instead of
    being suppressed."""
    leftover = guard.kill_and_reap_remaining(deadline_seconds=15.0)
    if leftover:
        raise AssertionError(f"selftest stranded descendants: {leftover}")


def reap_own_children() -> None:
    """Kept for call-site clarity: the kill+reap drain above already waited."""
    guard.reap_adopted_zombies()

WATCHDOG_SRC = (Path(__file__).resolve().parents[1] / "watchdog" / "pgid-watchdog-v3.sh")
COEFF_BYTES = 8192

FAKE_SOLVER = r'''
import hashlib, json, os, sys, time
from pathlib import Path

t0 = time.monotonic()
output = Path(sys.argv[2])
barrier = Path(os.environ["NSBU_N512_TEMPORAL_BARRIER_DIR"])
nonce = os.environ["NSBU_N512_TEMPORAL_BARRIER_NONCE"]
secret = os.environ["NSBU_N512_TEMPORAL_BARRIER_SECRET"]
clock = int(os.environ["NSBU_N512_TEMPORAL_BARRIER_CLOCK"])
deadline = int(os.environ["NSBU_N512_TEMPORAL_BARRIER_DEADLINE"])
coeff_bytes = int(os.environ["FAKE_SOLVER_COEFF_BYTES"])
tamper = os.environ.get("FAKE_SOLVER_TAMPER") == "1"
rearm = os.environ.get("FAKE_SOLVER_REARM") == "1"
identity = os.environ["FAKE_SOLVER_IDENTITY"]

def derive(action):
    msg = (action + "\n" + nonce + "\n" + str(clock) + "\n" + state + "\n").encode()
    key = bytes.fromhex(secret)
    return hashlib.sha256(key + msg + key).hexdigest()

output.mkdir()
(output / "rest.json").write_text(json.dumps(
    {"schema": "p10-avx-n384-rest-v1", "clock": 0, "state_payload": False,
     "observation_status": "RestExact", "balance": "REST"}))
step = output / "step-001-clock-0032"
step.mkdir()
coeffs = bytes(((index % 253) + 1 for index in range(coeff_bytes)))
state = hashlib.sha256(coeffs).hexdigest()
attempt = {"schema": "p10-avx-scheduled-attempt-v3", "identity": identity,
           "attempt": 1, "attempted_from": 0, "attempted_to": 32, "ticks": 32,
           "outcome": "committed", "rhs_calls": 11 if tamper else 12,
           "cache_hits": 7, "cache_misses": 5, "integration_seconds": 900.1,
           "rhs_evaluate_seconds": 800.0, "rhs_timed_calls": 12,
           "outside_rhs_evaluate_seconds": 100.1, "observer_seconds": None,
           "error_ratio_l2": 1.58e-8, "error_ratio_h1": 4.32e-8,
           "steady_allocations": 0}
(step / "attempt.json").write_text(json.dumps(attempt))
(step / "record.json").write_text(json.dumps(
    {"schema": "p10-avx-n512-m512-h32-observer-state-v1", "identity": identity,
     "resumable": False, "clock": 32, "epoch": 1, "accepted_steps": 1,
     "coefficient_bytes": coeff_bytes, "state_sha256": state,
     "observation_status": "CapturedActualState", "offline_observer_node": False,
     "observer_execution": "offline-baccus-required", "qualification": False}))
with (step / "state.bin").open("wb") as stream:  # writer-exact layout, never sparse
    stream.write(b"P10AVXSNAP1\0")
    identity_bytes = identity.encode()
    stream.write(len(identity_bytes).to_bytes(8, "little") + identity_bytes)
    for word in (32, 4096, 1, 1):
        stream.write(word.to_bytes(16, "little"))
    stream.write(coeffs)
    stream.write(hashlib.sha256(coeffs).digest())
print("attempt=1 clock=32 integration_seconds=900.100000000 rhs_evaluate_seconds=800.0 "
      "rhs_timed_calls=12 outside_rhs_evaluate_seconds=100.1 cache_hit_miss=[7, 5] "
      "observer_seconds=None ratios=[1.5877938231527918e-8, 4.324683895722008e-8] "
      "publication=Step state_sha256=Some(\"" + state + "\") steady_allocations=0",
      flush=True)
(barrier / "_latency").write_text("%.6f" % (time.monotonic() - t0))
fd = os.open(str(barrier / "barrier-armed.json"), os.O_WRONLY | os.O_CREAT | os.O_EXCL, 0o644)
with os.fdopen(fd, "w") as stream:
    stream.write(json.dumps({"schema": "p10-avx-n512-temporal-barrier-armed-v1",
                             "attempt": 1, "clock": clock, "nonce": nonce,
                             "state_sha256": state, "deadline_epoch": deadline,
                             "qualification": False}))
print("temporal_barrier armed attempt=1 clock=32 state_sha256=" + state
      + " deadline=" + str(deadline), flush=True)
stop = time.time() + (deadline - time.time())
while time.time() < stop:
    for name, action in (("abort.token", "abort"), ("release.token", "release")):
        path = barrier / name
        if path.exists() and path.read_text().strip() == derive(action):
            if action == "abort":
                print("barrier_aborted_no_attempt2", flush=True)
                raise SystemExit(97)
            if rearm:
                # Emulate a BUGGY solver that would arm again at clock 64:
                # the one-shot Rust barrier forbids this, and the supervisor
                # must treat the second armed line as a protocol violation.
                print("temporal_barrier armed attempt=1 clock=64 state_sha256=" + state
                      + " deadline=" + str(deadline), flush=True)
                print("one_shot_violation_simulated", flush=True)
                raise SystemExit(99)
            clock_now = 32
            for index in range(2, 97):
                clock_now += 32 if clock_now < 2048 else 64
                bundle = output / ("step-%03d-clock-%04d" % (index, clock_now))
                bundle.mkdir()
                (bundle / "record.json").write_text("{}")
                (bundle / "attempt.json").write_text("{}")
                with (bundle / "state.bin").open("wb") as stream:
                    stream.write(b"placeholder " * 400)
            print("terminal endpoint_capture_complete_offline_observer_required clock=4096",
                  flush=True)
            raise SystemExit(0)
    time.sleep(0.2)
print("barrier_expired_no_attempt2", flush=True)
raise SystemExit(98)
'''


def build_stage(tmp: Path, plan: dict) -> None:
    stage = tmp / "stage"
    (stage / "logs").mkdir(parents=True)
    (stage / "barrier").mkdir()
    shutil.copy(WATCHDOG_SRC, stage / "pgid-watchdog-v3.sh")
    (stage / "preflight.stdout").write_bytes(b"selftest synthetic preflight\n")
    (tmp / "fake_solver.py").write_text(FAKE_SOLVER)
    real_solver = stage / "solver"
    real_solver.write_text(f'#!/bin/sh\nexec python3 "{tmp}/fake_solver.py" "$@"\n')
    real_solver.chmod(0o755)
    plan["binary_sha256"] = contract.sha256_file(stage / "solver")
    plan["watchdog_sha256"] = contract.sha256_file(stage / "pgid-watchdog-v3.sh")
    plan["preflight_sha256"] = contract.sha256_file(stage / "preflight.stdout")


def run_case(name: str, tamper: bool = False, rearm: bool = False,
             receipt_uncertain: bool = False) -> dict:
    tmp = Path(tempfile.mkdtemp(prefix=f"h32-selftest-{name}-"))
    try:
        plan = plan_template()
        plan["guard"]["absolute_deadline_epoch"] = 1789999999
        plan["capture"]["coefficient_bytes"] = COEFF_BYTES
        build_stage(tmp, plan)
        solver_path = tmp / "stage" / "solver"
        solver_path.write_text(f'#!/bin/sh\nexec python3 "{tmp}/fake_solver.py" "$@"\n')
        solver_path.chmod(0o755)
        plan["binary_sha256"] = contract.sha256_file(solver_path)
        raw = json.dumps({k: v for k, v in plan.items() if not k.startswith("_")},
                        indent=2).encode()
        (tmp / "stage" / "frozen-plan.json").write_bytes(raw)
        digest = contract.sha256_bytes(raw)
        import os

        os.environ[contract.OPTIN_ENV] = "1"
        os.environ[contract.REVIEWED_HOST_ENV] = "sulaco"
        os.environ["FAKE_SOLVER_IDENTITY"] = IDENTITY
        os.environ["FAKE_SOLVER_COEFF_BYTES"] = str(COEFF_BYTES)
        os.environ.pop("FAKE_SOLVER_TAMPER", None)
        os.environ.pop("FAKE_SOLVER_REARM", None)
        if tamper:
            os.environ["FAKE_SOLVER_TAMPER"] = "1"
        if rearm:
            os.environ["FAKE_SOLVER_REARM"] = "1"
        driver.real_hostname = lambda: "sulaco"
        # The HOST process-exclusivity census is proven separately (real
        # refusal evidence + unit tests); the synthetic selftest stage runs
        # on a shared build host, so the census itself is stubbed to keep
        # the case deterministic — every other reading stays real.
        driver.host_process_observations = lambda *_a, **_k: []
        driver.real_mem_available_bytes = lambda *a, **k: 250_000_000_000
        driver.real_disk_free_bytes = lambda *_a, **_k: 400_000_000_000
        argv = ["launch", "--stage", str(tmp / "stage"), "--plan-sha256", digest]
        import h32_run as flow_module

        original_create_json = flow_module.receipts.create_json
        if receipt_uncertain:
            def spoofed_create_json(path, payload, mode=0o644):
                if Path(path).name == "decision-receipt.json":
                    return "uncertain_exists"
                return original_create_json(path, payload, mode)
            flow_module.receipts.create_json = spoofed_create_json
        try:
            code = driver.main(argv)
        finally:
            flow_module.receipts.create_json = original_create_json
        stage = tmp / "stage"
        receipts = sorted(p.name for p in stage.glob("*receipt.json"))
        barrier_dir = stage / "barrier"
        tokens = sorted(p.name for p in barrier_dir.iterdir()
                        if p.name.endswith(".token") or p.name.endswith(".json"))
        latency_file = barrier_dir / "_latency"
        latency = float(latency_file.read_text()) if latency_file.exists() else None
        decision_path = stage / "decision-receipt.json"
        elapsed = (json.loads(decision_path.read_text()).get("first_step_elapsed_seconds")
                   if decision_path.exists() else None)
        failure_path = stage / "failure-receipt.json"
        failure = (json.loads(failure_path.read_text())
                   if failure_path.exists() else {})
        output = stage / "output"
        captures = sum(1 for p in output.iterdir() if p.is_dir()
                       and p.name.startswith("step-")) if output.exists() else 0
        return {"code": code, "receipts": receipts, "tokens": tokens,
                "captures": captures, "latency": latency,
                "first_step_elapsed": elapsed, "failure_stage": failure.get("stage"),
                "failure_reasons": failure.get("reasons") or failure.get("reason")}
    finally:
        sweep_own_process_groups()
        reap_own_children()
        shutil.rmtree(tmp, ignore_errors=True)


def release_case_ok(positive) -> bool:
    return (
        positive["code"] == 0
        and "completion-receipt.json" in positive["receipts"]
        and "launch-receipt.json" in positive["receipts"]
        and "decision-receipt.json" in positive["receipts"]
        and "release.token" in positive["tokens"]
        and positive["tokens"] == ["barrier-armed.json", "release.token"]
        and positive["captures"] == 96
        and positive["first_step_elapsed"] is not None
        and positive["first_step_elapsed"] >= positive["latency"]
    )


def main() -> int:
    positive = run_case("release")
    abort = run_case("abort", tamper=True)
    rearm = run_case("rearm", rearm=True)
    uncertain = run_case("decision-uncertain", receipt_uncertain=True)
    ok = (
        release_case_ok(positive)
        and abort["code"] in (76, 79)
        and "abort.token" in abort["tokens"]
        and abort["tokens"] == ["abort.token", "barrier-armed.json"]
        and "failure-receipt.json" in abort["receipts"]
        and "decision-receipt.json" in abort["receipts"]
        and abort["captures"] == 1
        and rearm["code"] != 0
        and "completion-receipt.json" not in rearm["receipts"]
        and rearm["tokens"] == ["barrier-armed.json", "release.token"]
        and rearm["failure_stage"] == "barrier_one_shot_violation"
        and uncertain["code"] == 79
        and uncertain["tokens"] == ["abort.token", "barrier-armed.json"]
        and uncertain["failure_stage"] == "barrier_decision"
        and any("decision_receipt_not_durable" in r
                for r in uncertain["failure_reasons"])
        and "decision-receipt.json" not in uncertain["receipts"]
        and "completion-receipt.json" not in uncertain["receipts"]
        and uncertain["captures"] == 1
    )
    print("positive:", positive)
    print("abort:", abort)
    print("rearm:", rearm)
    print("decision-uncertain:", uncertain)
    print("SELFTEST PASS" if ok else "SELFTEST FAIL")
    return 0 if ok else 1


if __name__ == "__main__":
    raise SystemExit(main())
