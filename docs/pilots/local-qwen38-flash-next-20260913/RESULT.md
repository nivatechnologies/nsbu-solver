# Local Qwen pilot result

The local endpoint served the requested `qwen3.8-flash-next` model and sustained
eight simultaneous inference requests with no queued requests in the captured
telemetry. Synthetic nonce readback succeeded through both Chat Completions and
Responses. These checks establish endpoint availability, concurrency and basic
tool-result transport only.

Seven native read-only Codex audits each reached their 900-second limit without
a final report. Their transcripts contain useful source navigation but are not
completed audits. The native implementation task ran for about 27 minutes,
wrote a late isolated harness, and was stopped without a handoff or test receipt.
Review found compile and arithmetic-contract defects, so none of that source was
integrated. The detailed cloud review is preserved in the
[audit receipt](../../../evidence/pilot-local-qwen38/native-audit-review-20260913/ROOT_REVIEW.md),
whose SHA-256 is `e558d22ab5c5b8411e85cda110e556e5f268b6cc10680c42bb1ab0392b35c736`.
The native implementation transcript remains private and is bound by hashes in
[its failure receipt](../../../evidence/pilot-local-qwen38/native-implementer-failure.json).

A fixed-input, tool-free Chat Completions task was more effective. Its first
code response arrived in 31.61 seconds but failed compilation. One bounded
repair arrived in 32.19 seconds and passed six embedded tests. Independent test
generation then needed two bounded repairs: the first suite did not compile,
the next passed 22 of 23 tests but had one wrong oracle, and the final run
passed six embedded plus 17 independent tests. The externally reviewed result implements empirical
binary64 triangle-bound arithmetic; it is neither a certified enclosure nor a
regional or PDE validation result. Exact prompts, outputs, failures, repaired
source and test receipts are in the [direct pilot evidence](../../../evidence/pilot-local-qwen38/direct-bounds-20260913/README.md).

This pilot does not establish the proposed 99% local-agent workflow. The next
pilot should use narrow fixed-input deliverables, preconfigured writable build
directories, short wall limits, exact result/error contracts and bounded repair
turns. Scale beyond eight concurrent requests only after those tasks produce
timely final receipts and pass independent checks. Cloud review retains gate
authority, and automatic provider fallback remains disabled.
