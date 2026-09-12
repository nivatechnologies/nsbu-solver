# Scheduled AVX/reduced N192 endpoint experiment

This separately identified experimental harness owns a from-rest N192 trajectory with M384
integration-force sampling, 32 persistent reduced-force workers, the RustFFT 6.4.1 AVX/FMA
backend, Cox--Matthews h32 attempts, the unchanged local tolerances, and active-stage advective
guard 0.45. It stops after at most 128 committed attempts at the first legal clock 4096.

The current doubled-force-grid observer is intentionally evaluated only at the immutable positive
nodes 512, 1024, 1536, 2048, 2560, 3072, 3584 and 4096. Exact rest supplies node zero and
`BalanceSample::REST`. These nine nodes form fixed nested composite-Simpson sets with spacings
2048, 1024 and 512. No quadrature or spatial sufficiency follows merely from completing them.

Every accepted proposal is observed when scheduled and serialized while `PreparedCommit` still
protects the borrowed candidate. The complete attempt record is staged with it before the
infallible in-memory swap. Unscheduled attempts publish one atomic record after the swap. At a
scheduled clock, `state.bin`, `record.json`, and `attempt.json` publish together through one node
directory rename after the swap. This is a deliberate commit-then-publish protocol, not a
crash-atomic memory/filesystem transaction.

The failure record distinguishes the attempted index, in-memory committed clock, last durable
node clock, last durable attempt record, and any staged provisional clock. Observer or staging
failure drops `PreparedCommit`, leaves the committed state unchanged, and removes incomplete
staging. A post-commit rename or directory-sync failure retains the provisional bundle for audit,
marks qualification incomplete, and conservatively leaves the durable frontier unchanged. The
snapshot format is harness-owned, explicitly non-resumable, and cannot enter the v1 checkpoint
path. Rejected attempts retain their exact interval and both indicators; numerical errors retain
their exact interval and JSON-escaped cause.

The 96 GiB experimental cap and 4 GiB artifact cap are immutable parts of this profile. Preflight
includes both persistent force pools and their stacks/lanes, cached integration slots, all scalar
FFT workspaces, the shared catalog allowance, state/candidate/attempt storage, observer force and
conservative workspaces, file buffering, record allowances, and all nine snapshot payloads.

`frozen-plan.json` is the review contract. The source is frozen before the release binary embeds
its commit identity. Test evidence covers exact nesting/eight positive observations, disk-cap
refusal, overwrite refusal, JSON escaping, precommit observer/write failure without state or
frontier movement, and post-commit publication failure with distinct provisional/in-memory and
durable frontiers.

The first `f134856` pilot was stopped before attempt one after review found the earlier publication
ordering inadequate. Its exact rest node is retained as interrupted evidence and is not an endpoint
result.
