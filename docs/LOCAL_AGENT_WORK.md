# Local agent work policy

The Sparks should spend their inference capacity on useful, independently
checkable solver work. GPU utilization is an operating measurement, not a
success criterion. Repeated exploration, duplicate tasks, and a growing cloud
review queue are reasons to reduce dispatch, even when a GPU would otherwise
be idle.

This policy supersedes the execution recommendations in the historical
[first local pilot](pilots/local-qwen38-flash-next-20260913/RESULT.md). Its
failures and receipts remain historical evidence. Numerical acceptance rules,
frozen inputs, and package exit requirements are unchanged.

## Worker contract

- Use the explicitly configured local endpoint and model with reasoning enabled.
  There is no automatic cloud fallback, model-server reconfiguration, or SSH.
- Receive a bounded task packet: source identities, named input artifacts, exact
  deliverable contract, trusted validator, and explicit time/tool/repair limits.
- Read only named artifacts during the tool phase. The controller enforces the
  tool-call budget; a sentence asking the model to stop is insufficient.
- Reserve time for a separate terminal request. Omit tool definitions and tool
  choice entirely in this request. Preserve relevant evidence and require the
  contracted structured result.
- Validate schema, provenance, and task-specific behavior before promoting a
  result. A successful HTTP response or normal stop reason is not task success.
- Feed concrete validator failures into at most two local repair attempts.
  Preserve each failed attempt. Exhaustion creates a compact escalation receipt;
  it does not automatically summon a cloud model or change the contract.

## Dispatch and backpressure

Eight concurrent local workers are the initial capacity ceiling. Twelve is an
available configuration ceiling, not an established useful operating point.
Increase concurrency only after measuring useful completions and review burden,
not merely successful API requests.

The queue contains real dependency-ready work. It does not manufacture work to
fill slots or repeatedly retry completed task identifiers. Persist outcomes and
attempt counts across restarts.

Reserve review capacity before dispatching tasks that need human or cloud
review. The initial cap is two review-required tasks, counting both running
tasks and finished candidates awaiting review. This prevents an eight-worker
burst from silently creating eight reviews. Other slots may serve tasks whose
trusted, task-specific validators can complete routine artifact checks without
manual review. A packet cannot exempt itself from review through model output.

Pause a task family after three consecutive terminal failures. Other ready
families may continue. Report local repair frequency as well as terminal
failures: a class that succeeds only after frequent repairs may still have poor
useful throughput. Invalid or unverifiable scientific claims never become
acceptable merely because a repair budget has been exhausted.

Default work windows are bounded to two hours, with durable receipts and queue
state at the checkpoint. Empty queues, budget exhaustion, review backpressure,
and task failures are different states and must be reported separately.

## What validation means

A validated code candidate is ready for review. It is not automatically merged,
published, or scientifically accepted. A checked fixture or derived diagnostic
artifact may finish automatically only under a trusted validator authorized for
that scope. Validators must check the actual artifact rather than trusting an
agent's self-reported test result.

PDE trajectories, numerical-budget changes, acceptance decisions, and reviewed
input changes retain their existing gates. Local workers cannot reset an
integrated state to a reference solution, manufacture missing measurements, or
reinterpret a sampled diagnostic as an accepted window.

## Measurements at each checkpoint

Report completed useful tasks, first-attempt successes, local repairs, terminal
failures, outstanding reviews, and blocked task families. Bind results to source
and input hashes. Record request latency, token usage where available, validator
time, and the actual stop reason. Record cloud/human review time when measured;
otherwise report it as unmeasured, never as zero.

Observe GPU utilization and throughput separately from queue occupancy and
KV-cache utilization. Low KV-cache use is not low physical memory use: the model
server may preallocate most device/unified memory. Transient input-processing
queues are distinct from out-of-memory failures. The controller does not tune
GPU or model-server settings automatically.

Before making a broad local-autonomy claim, demonstrate that real task families
complete with low repair and review cost over successive work windows. The
earlier small successful fixture is evidence for a narrow workflow, not proof
that 99% of the project can already proceed autonomously.

## First bounded workload evidence

The eight-task modal fixture batch completed in 166.94 seconds: eight first-pass
successes, 312 independently checked integer derivative entries, no local
repairs, and no pending fixture reviews. Both GPUs averaged about 92% utilization
in this short run. A separate native named-artifact call and tool-free terminal
report also passed. See the [preserved receipts and limitations](../evidence/p10/local-modal-derivative-fixtures-20260913/README.md).

A larger Rust consumer generation trial exhausted its output and time budgets;
no code was accepted. That task class was paused rather than repeatedly retried.
These results support narrow fixture delegation, not general autonomous coding
or PDE acceptance. Review time remains unmeasured.

## Current task sizing

The current local deployment admits eight simultaneous sequences. Its 262,144
context tokens include prompt and generated output. The 8,192-token scheduler
iteration budget uses chunked prefill; it is not the maximum prompt length or
an output cap. Keep the operating concurrency at eight for this deployment.

Reasoning consumes the client output budget. Use about 8,192 output tokens for
compact fixture packets and 16,384–32,768 for bounded coding packets, with request timeouts sized for concurrent decode: 600 seconds for 8K,
900 for 16K, and 1,800 for 32K output tokens. Coding tasks have an explicit
1,800–3,600-second total limit and a final-report reserve sized for their output.
Supply the relevant interfaces and contract rather than an unrelated full file.
The transport byte cap covers the entire response, including reasoning: allow
sufficient space (for example 524,288 bytes for a 32K-token coding request), then
limit the final code artifact separately in its trusted validator. These are
task settings, not permission to increase tool, repair or review budgets.

Report actual endpoint activity. A prepared packet is not a running agent, and
zero running/waiting requests must not be explained away as prefill without
evidence. Idle time after a completed queue is distinct from inference failure.

At an observed aggregate 220–250 tokens/second across eight equally loaded
requests, generating 32K tokens per request alone takes approximately 18–20
minutes. Prompt processing adds time. A larger output cap without a matching
wall-clock budget merely changes truncation into timeout. Actual short responses
finish early; these limits do not require filling the output allowance.
