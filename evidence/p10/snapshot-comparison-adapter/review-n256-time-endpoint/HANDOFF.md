# N256 endpoint time diagnostic: review handoff

Status: both endpoints complete; manifests prepared for root review; comparator
not invoked. This package requests only a `TIME_DIAGNOSTIC` comparison with cap
812,646,400 bytes. It does not assess acceptance and does not support trajectory
injection or resume.

Both snapshots are real completed N256/M384 states at physical clock 4096 with
target 8192, case `e1236f...68f7e`, quantum exponent -20, unit domain lengths,
viscosity 1, Cox–Matthews, and the same tolerance bits. The left state uses h32
for 128 accepted steps and guard 0.45/max128. The right state uses h64 through
clock 2048 and h128 through 4096 for 48 accepted steps and guard 3.3/max48. The
manifests preserve those differences independently.

Both identities publish profile fields. The left binding is exactly
`n256-m384` from source `81bd07ec0e52a16044e2814bf0dbf9711669830e`.
The right binding is exactly
`n256-m384-h64to2048-h128to4096-cadv33-w3-f13c29c` from source
`569fd0ced7a755b1f6c066a7c00016f537fbc4bf`.

The arithmetic review is source-specific. The measured N256/M384 startup-rest
exact-bit controls used serial source
`8f579375d2d5ad79a573880393c05a297acf98d5` and final integrated W3 source
`f13c29c9ae91d0b8cf7a790132deb9bd076911c0`, producing the same state SHA-256
`a36034adc5665a950b33252691a5acc2c6c22ea61ad3304ba19b542b61dcf054`.
Git ancestry places the left endpoint source before that serial control and the
W3 control before the right endpoint source; the endpoint identities bind the
corresponding serial and W3 execution paths. This supports only the recorded
`reviewed-equivalence-supported-by-controls` lineage for a time/execution
diagnostic. The control archive says it is startup-state evidence and is not
later-state or arithmetic qualification. Neither current endpoint source is
relabeled as a directly measured control source.

`review-provenance.json` binds both endpoint records, full files, coefficient
payloads, plans, clock headers, control records, and lineage records. The
manifests use absolute paths to the immutable files in their originating
isolated worktrees. Root review must approve the manifests and
`arithmetic-review.json` before any adapter command is run. No comparison output
exists in this directory.
