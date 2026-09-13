# Bounded local agent queue

The operating and review policy is documented in
[`docs/LOCAL_AGENT_WORK.md`](../../docs/LOCAL_AGENT_WORK.md).

This controller sends fixed JSON task packets to one explicitly configured local
OpenAI-compatible endpoint. It has no endpoint default, credential handling,
cloud fallback, daemon mode, shell tool, repository writer, commit action, or
scientific acceptance action.

Run it explicitly with:

```text
python -m tools.local_agents --config config.json --tasks tasks.json --state work/queue.json
```

The config accepts `endpoint` and `model`, or those two values may come from
`LOCAL_QWEN_ENDPOINT` and `LOCAL_QWEN_MODEL`. Other fields use the checked
`RunnerConfig` defaults. A task packet contains `task_id`, `family`,
`instructions`, JSON `inputs`, a JSON-schema-like `output_contract`, a registered
`validator` name, and optional `artifacts` mapping names to readable files.
`review_required` defaults to true. Only validators explicitly trusted by the
controller may disable review.

Inference has two phases. The first exposes only `read_artifact`, which reads a
named, declared artifact under a byte cap. The final report request omits the
`tools` field. Every request enables thinking. Final JSON must pass the declared
required-field and primitive property-type subset of JSON Schema and its
deterministic validator; unsupported schema keywords have no effect. Validator errors
are supplied verbatim to at most two repair attempts. Receipts keep input/output
hashes, attempts, timing, token counts, and outcomes. `validated_candidate` is
always distinct from `scientific_accepted`, which the runner leaves false.

Each transport call runs behind an outer wall-clock guard using the remaining
task and run budget. A timed-out transport thread is daemonized and cannot hold
queue shutdown open; the HTTP socket timeout remains as its cleanup bound.
The tool phase stops before a separately configured final-report time reserve;
the final request can use that reserve up to the full task deadline.

The queue reserves at most two pending human-review slots by default, pauses a
family after three consecutive failed receipts, skips terminal duplicates, and
saves state atomically after results and at shutdown. Empty queues do no work.
The run has a two-hour default ceiling and never starts automatically.
