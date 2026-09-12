# Scheduled AVX/reduced N192 endpoint experiment

This separately identified experimental harness owns a from-rest N192 trajectory with M384
integration-force sampling, 32 persistent reduced-force workers, the RustFFT 6.4.1 AVX/FMA
backend, Cox--Matthews h32 attempts, the unchanged local tolerances, and active-stage advective
guard 0.45. It stops after at most 128 committed attempts at the first legal clock 4096.

The current doubled-force-grid observer is intentionally evaluated only at the immutable positive
nodes 512, 1024, 1536, 2048, 2560, 3072, 3584 and 4096. Exact rest supplies node zero and
`BalanceSample::REST`. These nine nodes form fixed nested composite-Simpson sets with spacings
2048, 1024 and 512. No quadrature or spatial sufficiency follows merely from completing them.

Every accepted attempt publishes one harness-schema record. At scheduled nodes, the observer and
snapshot finish before the node directory and attempt record are atomically renamed into place.
A failed diagnostic or write publishes no partial node, writes `qualification-incomplete.json`,
and leaves the last complete node as the artifact frontier. The snapshot format is harness-owned,
explicitly non-resumable, and cannot enter the v1 checkpoint path.

The 96 GiB experimental cap and 4 GiB artifact cap are immutable parts of this profile. Preflight
includes both persistent force pools and their stacks/lanes, cached integration slots, all scalar
FFT workspaces, the shared catalog allowance, state/candidate/attempt storage, observer force and
conservative workspaces, file buffering, record allowances, and all nine snapshot payloads.

`frozen-plan.json` is the review contract. The source is frozen before the release binary embeds
its commit identity. Test evidence covers exact nesting/eight positive observations, disk-cap
refusal, overwrite refusal, and injected write failure with no partial publication.
