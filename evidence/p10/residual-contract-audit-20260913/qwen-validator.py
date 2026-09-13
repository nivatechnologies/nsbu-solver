from __future__ import annotations

import json
from pathlib import Path

from tools.local_agents.__main__ import load_config, load_tasks
from tools.local_agents.contracts import JsonValue
from tools.local_agents.runner import QueueRunner

HERE = Path(__file__).resolve().parent
ALLOWED = {
    "IMPLEMENTATION_PLAN.md:178-180", "IMPLEMENTATION_PLAN.md:243-249",
    "docs/design/COMPLETE_DESIGN.md:378-384", "docs/design/COMPLETE_DESIGN.md:398-413",
    "docs/NEXT_CONCENTRATING_WINDOW.md:130-136", "docs/NEXT_CONCENTRATING_WINDOW.md:217-238",
    "crates/nsbu-solver/src/verification/budget.rs:7-60",
    "crates/nsbu-solver/src/verification/observation.rs:20-48",
    "crates/nsbu-solver/src/verification/review.rs:11-21",
    "crates/nsbu-solver/src/verification/review.rs:119-136",
    "evidence/p10/n384-offstage-hermite-design-20260913/design.json:3-7",
    "evidence/p10/n384-offstage-hermite-design-20260913/design.json:64-75",
}


def validate(output: dict[str, JsonValue], _: dict[str, JsonValue]) -> list[str]:
    errors: list[str] = []
    if set(output) != {"findings", "blocking_check", "implemented_weighting", "residual_to_velocity_bound"}:
        errors.append("wrong top-level fields")
    findings = output.get("findings")
    if not isinstance(findings, list) or len(findings) != 3:
        errors.append("findings must be a three-item array")
    else:
        kinds = set()
        for item in findings:
            if not isinstance(item, dict) or set(item) != {"kind", "claim", "citations"}:
                errors.append("malformed finding")
                continue
            kind, claim, citations = item.get("kind"), item.get("claim"), item.get("citations")
            if not isinstance(kind, str) or not isinstance(claim, str) or not isinstance(citations, list):
                errors.append("finding field types invalid")
                continue
            kinds.add(kind)
            if not citations or any(not isinstance(c, str) or c not in ALLOWED for c in citations):
                errors.append("citation outside supplied exact ranges")
        if kinds != {"raw_acceleration_h1", "temporal_residual_integration", "verifier_contract"}:
            errors.append("missing required finding kind")
    for field in ("blocking_check", "implemented_weighting", "residual_to_velocity_bound"):
        if not isinstance(output.get(field), str) or not output[field].strip():
            errors.append(f"{field} must be a nonempty string")
    corpus = json.dumps(output).lower()
    if "1933" in corpus and ("velocity h1 error" in corpus or "velocity error" in corpus):
        errors.append("must not convert 1933 to a velocity error")
    return errors


def main() -> None:
    config = load_config(HERE / "config.json", HERE / "state.json")
    tasks = load_tasks(HERE / "tasks.json")
    receipts = QueueRunner(config, validators={"residual_contract": validate}).run(tasks)
    (HERE / "summary.json").write_text(json.dumps({"receipts": receipts}, indent=2) + "\n")


if __name__ == "__main__":
    main()
