# N192 endpoint time diagnostic: review handoff

Status: prepared for root review; comparator not invoked. This package requests
only a `TIME_DIAGNOSTIC` comparison. It does not assess acceptance and does not
support trajectory injection or resume.

Both snapshots are real completed N192/M384 states at physical clock 4096 with
target 8192, case `e1236f...68f7e`, quantum exponent -20, unit domain lengths,
viscosity 1, Cox–Matthews, and the same tolerance bits. The legacy left state
uses h32 for 128 accepted steps and guard 0.45/max128. The right state uses h64
through clock 2048 and h128 through 4096 for 48 accepted steps and guard
3.3/max48. The manifests preserve those differences independently.

The left profile binding is deliberately `legacy-full-identity`: source
`92effa6068d20d69e815c6a83f1e82490ce37fe7` published no `profile=` field, so
the binding value is the complete state identity without an invented suffix.
The right binding is `identity-profile-field` and exactly matches the profile
field published by source `569fd0ced7a755b1f6c066a7c00016f537fbc4bf`.

The arithmetic review separates measured controls from current endpoint
lineage. The measured exact-bit outcome is the N256/M384 startup-rest serial/W3
control: the serial runs used source
`8f579375d2d5ad79a573880393c05a297acf98d5`; the final integrated W3
confirmation used `f13c29c9ae91d0b8cf7a790132deb9bd076911c0`; all report state
SHA-256 `a36034adc5665a950b33252691a5acc2c6c22ea61ad3304ba19b542b61dcf054`.
The control is startup-state evidence and its archive explicitly says it is not
later-state or arithmetic qualification. The review therefore concludes only
`reviewed-equivalence-supported-by-controls` for this time/execution diagnostic;
it does not relabel either endpoint source as directly measured exact-bit.

Specific review inputs are:

- `evidence/p10/avx-w3-n256-integration-20260912/summary.json`, SHA-256
  `6340a01b2324d8f30caf619bd087c8714b6a12a2c1065b3f7eb789ff9c8e881c`.
- `evidence/p10/avx-w3-n256-integration-20260912/runs/serial-1.stdout`, SHA-256
  `90d96b1e029487eebe930dec57681565868d7fc204795d52552dd5f851089311`.
- `evidence/p10/avx-w3-n256-integration-20260912/final-source/w3.stdout`, SHA-256
  `e1340e79300d6dcca39a80a7a7ea890f7f12719a4860de96ebc19743705f7676`.
- `evidence/p10/actual-state-smoke-92effa/batched-refinement.json` documents the
  reviewed scalar-source hash relationship from 92eff through the later serial
  source; its SHA-256 is recorded in this directory's `SHA256SUMS`.
- The right snapshot identity binds W3 source
  `f13c29c9ae91d0b8cf7a790132deb9bd076911c0` and reports serial RHS with W3
  reduced force. The exact endpoint record and frozen-plan hashes are also in
  `SHA256SUMS`.

The manifests refer to immutable state and plan files in their originating
isolated worktrees by absolute path. Root review should verify those paths and
hashes, inspect `arithmetic-review.json`, and approve the lineage before any
adapter command is run. The intended command, withheld here, would use cap
`344326144` bytes: two 171,638,784-byte coefficient payloads plus the fixed
1 MiB adapter overhead.
