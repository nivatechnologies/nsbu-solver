from __future__ import annotations

import json
from pathlib import Path

from tools.local_agents.__main__ import load_config, load_tasks
from tools.local_agents.contracts import JsonValue
from tools.local_agents.runner import QueueRunner

HERE = Path(__file__).resolve().parent
EXPECTED = """fn admit_layout(layout: Layout) -> Result<(), SolverError> {
    let dimensions = layout.dimensions();
    if matches!(
        dimensions,
        [6, 6, 6] | [384, 384, 384] | [512, 512, 512] | [576, 576, 576] | [768, 768, 768]
    ) {
        return Ok(());
    }
    Err(SolverError::InvalidPayload)
}"""


def normalized(source: str) -> str:
    return "".join(source.split())


def exact_768_replacement(output: dict[str, JsonValue], _: dict[str, JsonValue]) -> list[str]:
    if set(output) != {"replacement"} or not isinstance(output.get("replacement"), str):
        return ["output must contain exactly one string field: replacement"]
    replacement = output["replacement"]
    assert isinstance(replacement, str)
    return [] if normalized(replacement) == normalized(EXPECTED) else [
        "replacement must equal the complete original function plus only [768,768,768]"
    ]


def main() -> None:
    config = load_config(HERE / "config-v2.json", HERE / "state.json")
    tasks = load_tasks(HERE / "tasks-v2.json")
    runner = QueueRunner(config, validators={"exact_768_replacement": exact_768_replacement})
    receipts = runner.run(tasks)
    (HERE / "summary-v2.json").write_text(
        json.dumps({"schema": "p10-w3-admit-768-qwen-replacement-v2", "receipts": receipts}, indent=2) + "\n"
    )


if __name__ == "__main__":
    main()
