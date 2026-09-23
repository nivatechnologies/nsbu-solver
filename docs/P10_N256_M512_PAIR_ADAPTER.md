# P10 N256/M512 -> N384/M512 endpoint pair adapter

`crates/p10-n256-m512-pair-adapter` is a read-only, preparation-only adapter
for the conditional fixed-M512 spatial discriminant between the retained N256
and N384 branches at the shared clock-4096 trajectory endpoint. It is an
integration of an independently reviewed isolated package; it reuses, weakens
or rebinds nothing from the reviewed N384->N512
`MATCHED_M512_SPATIAL_DIAGNOSTIC` contract and emits an endpoint diagnostic
only. It performs no solver execution, staging, launch or qualification.

## What the implemented diagnostic is

Given an admitted, independently evolved N256/M512 coarse lineage and the
frozen reviewed `r6` N384/M512 endpoint, the adapter would compute one D1
band difference: the fine-minus-zero-extended-coarse comparison over the full
fine bandwidth on the fixed M512 integration grid, with the exact common-band
plus new-shell decomposition, preserved means and correct half-spectrum
weights and physical derivatives. The coarse branch is fixed at 32 steps of
64 through 2048 then 16 steps of 128 through 4096 with all 48 clocks scheduled
exactly; the fine side is bound byte-exactly to the frozen r6 identity,
backend and execution, and the endpoint file and coefficient hashes are bound
closed in `contract.rs`. Admission ordering is
deadline -> contract -> geometry -> lineage -> resource cap, and outputs keep
`acceptance.status = "not_assessed"`, `accepted_windows = 0`,
`endpoint_only = true` and `converged_pde_window = false`.

## Current refusal

Runtime comparison is refused today and this is the honest state, not a
placeholder:

- `reviewed_anchor()` in `lineage.rs` returns `None`. The reviewed N256
  capture preparation records `coarse_trajectory_executed: false` and
  `endpoint_attempts_executed: 0`, so no independently reviewed completed
  N256 lineage exists to authenticate against.
- Completed-lineage admission requires an out-of-band `TrustedLineage`
  receipt (pinned source/binary/plan/rest hashes and every committed step
  hash). It can never be satisfied, overridden or self-attested by the intake
  JSON itself; a self-consistent self-attested intake is reported
  `n256_m512_lineage_unverified`.
- The actual rest record is authenticated against the closed
  `p10-avx-n384-rest-v1` metadata schema and every committed state header is
  compared byte-for-byte against the reviewed lineage identity.

On the current vault both CLI modes refuse with exit 1:
`admit` reports `n256_m512_lineage_absent: no N256/M512 lineage intake exists`
and `compare` reports `n256_m512_lineage_absent` for the absent coarse input
manifest (`prepared_not_executed`). Usage:

```text
cargo run --release -p p10-n256-m512-pair-adapter -- admit LINEAGE_INTAKE.json
cargo run --release -p p10-n256-m512-pair-adapter -- compare COARSE.json FINE.json CAP_BYTES DEADLINE_EPOCH
```

## Provenance and limits

The integrated `src/` is byte-identical to the package sealed under
`SHA256SUMS` root `0ae6c40e2b133e2d8ced1da33bad2640d76298c4afe90940033795435ac61895`.
The independent adapter-preparation review report has SHA-256
`a1350fd43a2a3b924dddb793f55910fa33981454d2cbfb13745e82ec2d70bcf9`;
the separate workspace-integration review report has SHA-256
`1a4b5de5a3934f0316e733cc0e72051b14090181da367d94f09307ed55c2f1b2`.
Both reviews are scoped to preparation, with no comparison result, execution
authorization, refinement order or window acceptance. Integration changed only packaging
metadata: workspace-inherited manifest fields, pinned `serde`/`serde_json`
and the shared `sha2` workspace pin, plus this page and the
`p10_n256_m512_pair_adapter_preparation` status entry. A future execution
first requires an independently reviewed completed-lineage receipt compiled
in behind the anchor seam, its own resource/deadline enforcement and
authorization. One endpoint D1 alone cannot establish an observed order or
qualify `[0, 1/256]`; design sections 6.3-6.4 prerequisites remain unmet.
