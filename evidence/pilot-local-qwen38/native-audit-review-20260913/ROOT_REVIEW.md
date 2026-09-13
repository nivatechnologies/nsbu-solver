# Local audit pilot review

Reviewed 2026-09-13 by the cloud orchestrator. These are workflow observations,
not new PDE validation results.

Seven native read-only Codex tasks reached their 900-second caps without final
reports. Their command transcripts remain useful source-navigation records;
they are not completed audits. Endpoint telemetry independently demonstrated
eight simultaneous requests, zero queued requests, and about 12.3% KV-cache use.
Concurrency does not establish task quality or useful throughput.

Four later direct, tool-free Qwen requests summarized the captured transcripts
with thinking disabled. Triangle, time-admission, acceptance-gap, and arithmetic
summaries all returned text, but they remain partial syntheses of stale or
incomplete input. In particular:

- The acceptance summary incorrectly inferred that M512 had not launched from
  a deployment document despite later snapshot evidence. M512 actually stopped
  after durable clock 3072 under its continuation gate.
- A suggested 3/5/7 subset of observer nodes does not establish independent
  off-stage reconstruction or the required reviewed measurement geometry.
- A final force-pair comparison alone does not establish every force-related
  qualification requirement or acceptance of the window.
- Sparse high-precision reference samples establish discrepancies at those
  samples, not an upper bound over an entire grid, region, or time interval.
- Improving diagnostic reductions alone does not qualify arithmetic in the
  integrated trajectory, force evaluation, or FFTs.
- Admission-only independence of an accepted candidate requires the same
  numerical inputs and accepted schedule. It does not authorize changing
  archived metadata or assuming cross-binary reproducibility.
- The triangle summary describes a plan; it does not verify an implementation.

Use these summaries for navigation only. Do not copy their recommendations into
acceptance policy without checking the current source and evidence.

A separate synthetic nonce readback succeeded through Chat Completions and
Responses. The initial Responses extractor mistakenly included reasoning text;
the original result and the corrected output-text-only review are preserved in
`../qwen-protocol-readback-20260913/`. This small test provides no evidence of
basic tool-result loss, and does not prove compatibility of every native tool.

Next workflow: fixed source/input packets, narrow code or test deliverables,
bounded local repair iterations, explicit failure receipts, and independent
checks before integration. Do not claim 99% local autonomous delivery yet.
