# Working on NSBU Solver

Use the root [IMPLEMENTATION_PLAN.md](IMPLEMENTATION_PLAN.md) for package order
and [project-status.json](project-status.json) for recorded capability. Historical
plans under `docs/design/` are reviewed inputs, not active instructions.

Consult [architecture](docs/ARCHITECTURE.md) when changing component boundaries;
[numerical conventions](docs/NUMERICAL_CONVENTIONS.md) when changing numerical
kernels; [quality policy](docs/QUALITY.md) when selecting applicable gates; and
[next-window dependencies](docs/NEXT_CONCENTRATING_WINDOW.md) when qualifying PDE
windows. These are task-specific pointers, not a required reading sequence.
Documentation-only edits do not require loading the numerical design or running
PDE experiments.

Preserve reviewed files byte-for-byte, including `COMPLETE_DESIGN.md` revision
0.7 and `similarity-mms-v2.json`; use the repository checker for frozen hashes.
Keep the public library and CLI standalone. Comparison trajectories evolve
independently from rest: analytical references must never reset integrated state.
Preserve exact ticks, bounded attempts, resource preflight and transactional commits.
Passing algebra, unit tests, a grid screen or a trajectory run alone does not
qualify a PDE window. Completion and public claims require the package's actual
exit evidence; synchronize status and documentation when capability changes.

Within the user's authorized scope, carry implementation through relevant
verification and repair without stopping for routine approvals. Respect the
current work block's explicit deadline and leave time to preserve evidence and
stop owned jobs. A time allowance is a ceiling, not a requirement to consume it.
Do not infer remaining account credits from elapsed time or agent counts.

For delegated work, use [task prompts](docs/AGENT_TASKS.md); the current model
allocation and handoff policy are recorded there. Give each worker a bounded
outcome and exclusive edit ownership. Keep mutable run state, deadlines
and measurements in the task handoff/evidence rather than this file. Prioritize
work that resolves the current numerical bottleneck; broader infrastructure
needs a concrete dependency on that outcome.

Treat local Qwen3.8 27B as a narrow mechanical worker. Assign it bounded test
additions against an already fixed interface, inventories, documentation,
formatting, and deterministic evidence checks. Do not assign it architecture,
cross-module integration, concurrency or lifecycle ownership, transactional
publication, numerical-control decisions, launch orchestration, or repair of an
independently rejected candidate. Use Flash Next for those implementation tasks
and Astra for architecture and independent review. Prefer an idle 27B slot to a
candidate likely to create more review and repair work.

Worker timeouts are generous failsafes, not short work slices. Follow the sizing
and session-continuation guidance in [local agent work](docs/LOCAL_AGENT_WORK.md).
Review progress separately and preserve healthy workers across checkpoints.
