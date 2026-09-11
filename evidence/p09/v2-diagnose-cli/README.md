# Fixed-profile `diagnose-v2` CLI

This increment exposes the coordinator through `nsbu diagnose-v2 [--dry-run]`.
An owned `StartupProfile` holds the accepted, probe and residual clock arrays;
both the example and CLI borrow that same owner to admit the fixed N=4/8/12
profile. No arbitrary numerical options, concentrating windows or acceptance
policy are accepted.

Admission output identifies the mathematical case hash, family/probe identities,
six branch grid/step/method mappings, force grid and worker count, exact clocks,
three diagnostic sample grids, memory cap and component work preflight. The dry
run completes admission and exits before `DiagnosticDriver` allocation.

The actual process test retained seven concise JSONL summaries: three accepted
and four residual events. Accepted output keeps velocity, gradient, Hessian and
vorticity pair maxima separate, keeps pressure separate from pressure gradient,
and keeps the four analytical-reference quantity maxima separate. All accepted
nodes were bitwise equal. The largest residual L2 was the nonzero observed value
`5.1013585659162156e-2`; the test parses it and checks a finite bounded range
rather than relying on an exact formatted word. Every line remains labeled
`UnqualifiedDiagnostic`; the final record reports zero qualified windows.

The captured actual JSONL was produced after nested physical sampling merged and
before the subsequent worker metadata increment. The serial numerical default
was unchanged. The source was then rebased on worker main `7abac25`; final dry
run admission reports 69,490,624 joint bytes and 531,080 coordinator bytes.
Root owns the combined post-worker actual coordinator/allocator/CLI run.

Coverage includes the new profile and CLI production modules and the changed
coordinator example: 235/280 lines (83.93%) and 18/22 branches (81.82%). The
changed example has a cheap parser/admission orchestration test, so it is part of
the CRAP scope without duplicating a full PDE run. Maximum CRAP is 20. Across all
eight changed Rust files and 102 functions, the maxima are CC15, cognitive 12,
Halstead difficulty 40.6154 and 312 physical lines. The RCA source was parsed as
all 453 JSON-lines records. Strict workspace formatting/Clippy and exact strict
Rustdoc pass on the final source.

The five region classifier outputs are exclusive for each sampled point. Their
counts do not prove nominal-region coverage or volume enclosures. Full raw
library reports remain available and the CLI makes no completeness, convergence
or PDE qualification claim.
