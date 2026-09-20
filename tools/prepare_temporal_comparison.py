"""Prepare N512 h64/h32/h16 temporal comparison manifest inputs from captures.

Read-only and preparation-only: it validates the 24 captured observer records,
their inventories, capture plans and per-pair arithmetic reviews against the
closed nested-family contract and publishes pairwise TIME_DIAGNOSTIC comparison
input manifests plus exact temporal-order bookkeeping, create-only. It never
opens a state payload, never injects, resets or reference-substitutes an
independently evolved state, and its output is comparison evidence only: it
cannot accept a PDE window. Standard library only.
"""

from __future__ import annotations

import argparse
from collections.abc import Mapping, Sequence
import hashlib
import json
import os
from pathlib import Path
import sys
from typing import Final, TypeVar

if not __package__:
    sys.path.insert(0, str(Path(__file__).resolve().parents[1]))

from tools.json_types import Json, JsonObject, object_value
from tools.prepare_snapshot_manifest import publish_create_only
from tools.prepare_snapshot_manifest_contract import RECORD_FIELDS
from tools.prepare_snapshot_manifest_validation import (
    MAX_PLAN_BYTES, MAX_RECORD_BYTES, PreparationError, PublicationUncertainError, count_value,
    exact_keys, flag_value, hex_value, load_json_bounded, loads_checked, read_capped,
    string_value,
)
from tools.temporal_comparison_contract import (
    ABSOLUTE_TOLERANCES, ADAPTER_FIXED_OVERHEAD_BYTES, ADVECTIVE_LIMIT, BACKEND,
    BOOKKEEPING_SCHEMA, BRANCH_ORDER, CLOCK_TARGET, COEFFICIENT_BYTES, COMPARISON_SCHEMA_NAME,
    ENDPOINT_TICK, INVENTORY_SCHEMA, LENGTHS, PAIR_RESERVATION_BYTES, PAIRS, PAYLOAD_TOTAL_BYTES,
    PREPARATION_STATUS, QUANTUM_EXPONENT, RELATIVE_TOLERANCES, SHARED_CLOCKS, STATE_SCHEMA,
    SWITCH_CLOCK, TRUNCATION_NOTE, VISCOSITY, arithmetic_review_contract, attempts_at,
    branch_entries, bundle_name, emitted_family_plan, expected_state_file_bytes,
    schedule_through, validate_family_plan,
)

INVENTORY_NAME: Final = "state-inventory.json"
BOOKKEEPING_NAME: Final = "bookkeeping.json"
MAX_INVENTORY_BYTES: Final = 64 * 1024
OBSERVATION_STATUS: Final = "CapturedActualState"
STATE_NAME: Final = "state.bin"
RECORD_NAME: Final = "record.json"
ARITHMETIC_EVIDENCE_NAME: Final = "arithmetic-evidence.json"


def _sha256(data: bytes) -> str:
    return hashlib.sha256(data).hexdigest()


def _read_bounded(path: Path, limit: int, label: str) -> bytes:
    try:
        with open(path, "rb") as handle:
            raw = read_capped(handle, limit)
    except OSError as error:
        raise PreparationError(f"{label} could not be read: {error}") from error
    if len(raw) > limit:
        raise PreparationError(f"{label} exceeds its {limit} byte bound")
    return raw


def _require_present(path: Path, label: str) -> None:
    if not path.is_file():
        raise PreparationError(f"{label} is missing")


def _verify_plan_file(root: Path, name: str, entry: Mapping[str, Json]) -> str:
    plan_file = string_value(entry["plan_file"], f"branch {name} plan_file")
    path = root / name / plan_file
    _require_present(path, f"branch {name} capture plan {plan_file}")
    raw = _read_bounded(path, MAX_PLAN_BYTES, f"branch {name} capture plan")
    expected = hex_value(entry["plan_sha256"], 64, f"branch {name} plan_sha256")
    if _sha256(raw) != expected:
        raise PreparationError(f"branch {name} capture plan hash does not match plan_sha256")
    return expected


def _verify_inventory(root: Path) -> dict[str, str]:
    path = root / INVENTORY_NAME
    _require_present(path, "state inventory")
    values = load_json_bounded(path, MAX_INVENTORY_BYTES, "state inventory")
    exact_keys(values, frozenset({"schema", "files"}), frozenset(), "state inventory")
    if string_value(values["schema"], "inventory schema") != INVENTORY_SCHEMA:
        raise PreparationError(f"inventory schema is not {INVENTORY_SCHEMA}")
    files = values["files"]
    if not isinstance(files, dict):
        raise PreparationError("inventory files must be an object")
    expected_keys = {f"{name}/{bundle_name(name, clock)}/{STATE_NAME}"
                     for name in BRANCH_ORDER for clock in SHARED_CLOCKS}
    if set(files) != expected_keys:
        missing = sorted(expected_keys - set(files))
        extra = sorted(set(files) - expected_keys)
        raise PreparationError(f"inventory keys must be exactly the 24 shared captures; "
                               f"missing {missing}, unexpected {extra}")
    hashes = {key: hex_value(value, 64, f"inventory hash for {key}") for key, value in files.items()}
    if len(set(hashes.values())) != len(hashes):
        raise PreparationError("inventory whole-file hashes are not pairwise distinct")
    return hashes


def _verify_record(root: Path, name: str, clock: int, entry: Mapping[str, Json]) -> tuple[
        str, str, str, int]:
    bundle = bundle_name(name, clock)
    epoch = attempts_at(name, clock)
    directory = root / name / bundle
    if not directory.is_dir():
        raise PreparationError(f"branch {name} capture bundle {bundle} is missing")
    _require_present(directory / STATE_NAME, f"branch {name} {bundle} state payload")
    record_path = directory / RECORD_NAME
    _require_present(record_path, f"branch {name} {bundle} record")
    raw = _read_bounded(record_path, MAX_RECORD_BYTES, f"branch {name} {bundle} record")
    record = loads_checked(raw, f"branch {name} {bundle} record")
    exact_keys(record, RECORD_FIELDS, frozenset(), f"branch {name} {bundle} record")
    refusals = (
        (string_value(record["schema"], "record schema") == STATE_SCHEMA,
         f"branch {name} {bundle} record schema is not {STATE_SCHEMA}"),
        (record["identity"] == entry["identity"],
         f"branch {name} {bundle} identity does not match the immutable branch identity"),
        (not flag_value(record["resumable"], "record resumable"),
         f"branch {name} {bundle} must be resumable=false from rest"),
        (not flag_value(record["qualification"], "record qualification"),
         f"branch {name} {bundle} must be qualification=false"),
        (string_value(record["observation_status"], "record observation_status")
         == OBSERVATION_STATUS,
         f"branch {name} {bundle} is not a {OBSERVATION_STATUS} observer capture"),
        (flag_value(record["offline_observer_node"], "record offline_observer_node"),
         f"branch {name} {bundle} must mark offline_observer_node=true"),
        (record["observer_execution"] == entry["observer_execution"],
         f"branch {name} {bundle} observer execution conflicts with the branch plan"),
        (count_value(record["clock"], "record clock") == clock,
         f"branch {name} {bundle} is not the shared clock {clock}"),
        (count_value(record["epoch"], "record epoch") == epoch,
         f"branch {name} {bundle} epoch is not the exact schedule epoch {epoch}"),
        (count_value(record["accepted_steps"], "record accepted_steps") == epoch,
         f"branch {name} {bundle} accepted steps are not the exact {epoch} from-rest count"),
        (count_value(record["coefficient_bytes"], "record coefficient_bytes")
         == COEFFICIENT_BYTES,
         f"branch {name} {bundle} coefficient bytes are not the bound {COEFFICIENT_BYTES}"),
    )
    for passed, message in refusals:
        if not passed:
            raise PreparationError(message)
    return (_sha256(raw), hex_value(record["state_sha256"], 64, "record state_sha256"),
            bundle, epoch)


def _verify_arithmetic(root: Path, coarse: str, fine: str, binding_path: str,
                       plan: Mapping[str, Json]) -> tuple[Mapping[str, Json], str, str, bytes]:
    path = root / binding_path
    _require_present(path, f"pair {coarse}--{fine} arithmetic binding")
    raw = _read_bounded(path, MAX_PLAN_BYTES, f"pair {coarse}--{fine} arithmetic binding")
    control = object_value(arithmetic_review_contract(loads_checked(raw, "arithmetic binding"),
                                                      coarse, fine, plan))
    evidence = string_value(control["evidence"], "arithmetic evidence")
    evidence_path = path.parent / evidence
    _require_present(evidence_path, f"arithmetic evidence {evidence}")
    evidence_raw = _read_bounded(evidence_path, MAX_PLAN_BYTES, "arithmetic evidence")
    expected = hex_value(control["evidence_sha256"], 64, "arithmetic evidence_sha256")
    if _sha256(evidence_raw) != expected:
        raise PreparationError("arithmetic evidence hash does not match evidence_sha256")
    if loads_checked(evidence_raw, "arithmetic evidence") != control["review"]:
        raise PreparationError(
            "arithmetic evidence content does not match the embedded ArithmeticReview")
    return control, _sha256(raw), expected, evidence_raw


def _side_manifest(name: str, clock: int, entry: Mapping[str, Json], state_rel: str,
                   plan_rel: str, coefficient: str, file_hash: str,
                   control: Mapping[str, Json]) -> JsonObject:
    epoch = attempts_at(name, clock)
    return {
        "schema": COMPARISON_SCHEMA_NAME,
        "comparison_kind": "TIME_DIAGNOSTIC",
        "snapshot": state_rel,
        "plan": plan_rel,
        "identity": entry["identity"],
        "source_commit": entry["source_commit"],
        "plan_sha256": entry["plan_sha256"],
        "coefficient_sha256": coefficient,
        "file_sha256": file_hash,
        "backend": BACKEND,
        "execution": entry["execution"],
        "dimensions": [512, 512, 512],
        "evolution": {
            "case_sha256": entry["case"],
            "quantum_exponent": QUANTUM_EXPONENT,
            "clock_target": CLOCK_TARGET,
            "comparison_endpoint": clock,
            "lengths": list(LENGTHS),
            "viscosity": VISCOSITY,
            "method": "cox-matthews",
            "integration_force_dimensions": [512, 512, 512],
            "schedule": schedule_through(name, clock),
            "absolute_tolerances": [float(v) for v in ABSOLUTE_TOLERANCES],
            "relative_tolerances": [float(v) for v in RELATIVE_TOLERANCES],
        },
        "elapsed": clock,
        "target": CLOCK_TARGET,
        "epoch": epoch,
        "accepted_steps": epoch,
        "profile": {"kind": "identity-profile-field", "value": entry["profile"]},
        "admission_guard": {"advective_limit": ADVECTIVE_LIMIT,
                            "maximum_attempts": entry["maximum_attempts"]},
        "arithmetic_control": control,
    }


def _relate(target: Path, directory: Path) -> str:
    return os.path.relpath(target, directory).replace(os.sep, "/")


def _publish_all(outputs: Sequence[tuple[Path, bytes]]) -> None:
    for path, payload in outputs:
        os.makedirs(path.parent, exist_ok=True)
        publish_create_only(path, payload)


def prepare(plan_path: Path, root: Path, output: Path) -> JsonObject:
    plan_raw = _read_bounded(plan_path, MAX_PLAN_BYTES, "family plan")
    plan = validate_family_plan(loads_checked(plan_raw, "family plan"), strict=True)
    entries = branch_entries(plan)
    plan_hashes = {name: _verify_plan_file(root, name, entry)
                   for name, entry in entries.items()}
    inventory = _verify_inventory(root)
    captures: dict[tuple[str, int], tuple[str, str, str, int]] = {}
    for name, entry in entries.items():
        for clock in SHARED_CLOCKS:
            captures[(name, clock)] = _verify_record(root, name, clock, entry)
    coefficients = [value[1] for value in captures.values()]
    if len(set(coefficients)) != len(coefficients):
        raise PreparationError("coefficient hashes are not pairwise distinct across the 24 "
                               "independently evolved captures")
    if set(coefficients) & set(inventory.values()):
        raise PreparationError("coefficient and whole-file hash layers must be disjoint")
    arithmetic_files = {key: string_value(value, f"arithmetic binding {key}")
                        for key, value in object_value(plan["arithmetic_files"]).items()}
    controls: dict[str, tuple[Mapping[str, Json], str, str, bytes]] = {}
    for coarse, fine in PAIRS:
        controls[f"{coarse}--{fine}"] = _verify_arithmetic(root, coarse, fine,
                                                           arithmetic_files[f"{coarse}--{fine}"],
                                                           plan)
    pairs_root = output / "pairs"
    payloads: list[tuple[Path, bytes]] = []
    pair_rows: list[JsonObject] = []
    for coarse, fine in PAIRS:
        key = f"{coarse}--{fine}"
        control, _, _, evidence_raw = controls[key]
        side_control: JsonObject = {**control, "evidence": ARITHMETIC_EVIDENCE_NAME}
        for clock in SHARED_CLOCKS:
            directory = pairs_root / key / f"clock{clock:04d}"
            payloads.append((directory / ARITHMETIC_EVIDENCE_NAME, evidence_raw))
            side_rows: dict[str, str] = {}
            for name in (coarse, fine):
                entry = entries[name]
                _, coefficient, bundle, epoch = captures[(name, clock)]
                state_rel = _relate(root / name / bundle / STATE_NAME, directory)
                plan_rel = _relate(root / name / string_value(entry["plan_file"], "plan_file"),
                                   directory)
                manifest = _side_manifest(name, clock, entry, state_rel, plan_rel, coefficient,
                                          inventory[f"{name}/{bundle}/{STATE_NAME}"], side_control)
                payload = (json.dumps(manifest, indent=2) + "\n").encode("utf-8")
                payloads.append((directory / f"{name}.json", payload))
                relative = _relate(directory / f"{name}.json", output)
                side_rows[name] = relative
            pair_rows.append({
                "pair": key, "clock": clock, "coarse": coarse, "fine": fine,
                "coarse_epoch": attempts_at(coarse, clock), "fine_epoch": attempts_at(fine, clock),
                "manifests": side_rows, "reservation_bytes": PAIR_RESERVATION_BYTES,
            })
    ledger: list[JsonObject] = []
    for clock in SHARED_CLOCKS:
        sides: list[JsonObject] = []
        for name in BRANCH_ORDER:
            record_sha, coefficient, bundle, epoch = captures[(name, clock)]
            sides.append({
                "branch": name, "bundle": bundle, "epoch": epoch, "accepted_steps": epoch,
                "record_sha256": record_sha, "coefficient_sha256": coefficient,
                "file_sha256": inventory[f"{name}/{bundle}/{STATE_NAME}"],
                "state_file_bytes": expected_state_file_bytes(
                    string_value(entries[name]["identity"], "branch identity")),
            })
        ledger.append({"clock": clock, "sides": sides})
    bookkeeping: JsonObject = {
        "schema": BOOKKEEPING_SCHEMA,
        "status": PREPARATION_STATUS,
        "acceptance": {"status": "not_assessed", "accepted_windows": 0},
        "claim": ("comparison evidence only; this adapter cannot accept a PDE window, applies no "
                  "tolerance and changes none"),
        "family_plan_sha256": _sha256(plan_raw),
        "comparison_kind": "TIME_DIAGNOSTIC",
        "branch_order": list(BRANCH_ORDER),
        "pair_step_ratio": 2,
        "clock_target": CLOCK_TARGET,
        "trajectory_endpoint": ENDPOINT_TICK,
        "switch_clock": SWITCH_CLOCK,
        "shared_clocks": list(SHARED_CLOCKS),
        "schedule_truncation": TRUNCATION_NOTE,
        "ledger": ledger,
        "pairs": pair_rows,
        "arithmetic_bindings": {
            key: {"binding_sha256": binding, "evidence_sha256": evidence}
            for key, (_, binding, evidence, _) in controls.items()
        },
        "plan_files_sha256": plan_hashes,
        "disk": {
            "coefficient_bytes_per_state": COEFFICIENT_BYTES,
            "state_count": len(BRANCH_ORDER) * len(SHARED_CLOCKS),
            "coefficient_bytes_total": PAYLOAD_TOTAL_BYTES,
            "adapter_overhead_bytes_per_pair": ADAPTER_FIXED_OVERHEAD_BYTES,
            "pair_reservation_bytes": PAIR_RESERVATION_BYTES,
            "pair_invocations_total": PAIR_RESERVATION_BYTES * len(pair_rows),
            "state_file_bytes_by_branch": {
                name: expected_state_file_bytes(string_value(entry["identity"],
                                                             "branch identity"))
                for name, entry in entries.items()
            },
        },
        "independence": {
            "state_bytes_read_by_adapter": 0,
            "hash_provenance": ("carried through from capture records and the synced whole-file "
                                "inventory; never re-streamed here"),
            "state_reset_or_injection": "none",
            "reference_substitution": "none",
        },
    }
    payloads.append((output / BOOKKEEPING_NAME,
                     (json.dumps(bookkeeping, indent=2) + "\n").encode("utf-8")))
    _publish_all(payloads)
    return bookkeeping


_T = TypeVar("_T")


def _required(value: _T | None, name: str) -> _T:
    if value is None:
        raise PreparationError(f"missing required argument: {name}")
    return value


class Arguments(argparse.Namespace):
    family_plan: Path | None
    root: Path | None
    output: Path
    emit_family_plan: bool


def main(argv: Sequence[str] | None = None) -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--family-plan", type=Path, help="validated temporal family plan json")
    parser.add_argument("--root", type=Path, help="capture root holding the three branch trees")
    parser.add_argument("--output", type=Path, required=True, help="create-only output directory")
    parser.add_argument("--emit-family-plan", action="store_true",
                        help="write the closed family plan with explicit pending bindings")
    args = Arguments()
    parser.parse_args(argv, namespace=args)
    try:
        if args.emit_family_plan:
            os.makedirs(args.output.parent, exist_ok=True)
            publish_create_only(args.output,
                                (json.dumps(emitted_family_plan(), indent=2)
                                 + "\n").encode("utf-8"))
        else:
            prepare(_required(args.family_plan, "--family-plan"),
                    _required(args.root, "--root"), args.output)
    except PublicationUncertainError as error:
        print(f"publication uncertain: {error}", file=sys.stderr)
        return 2
    except (ValueError, OSError) as error:
        print(f"refused: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
