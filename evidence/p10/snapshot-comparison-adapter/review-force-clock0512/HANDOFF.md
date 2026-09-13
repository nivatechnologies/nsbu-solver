# N384 M384-to-M512 clock-512 force diagnostic review

Status: review only. The N384/M384 side is bound to the completed Sulaco
clock-512 state. The N384/M512 side is a deliberately non-runnable template
pending publication of its exact clock-512 record, state path, coefficient
SHA-256, and whole-file SHA-256. No comparison is authorized or has run.

The proposed `FORCE_RESOLUTION_DIAGNOSTIC` compares independent from-rest
prefixes through clock 512. Both retain N384 and use the exact prefix schedule
`[0,512)` at 64 ticks: eight committed steps and epoch 8 under the preserved
advective guard 3.3 and maximum-attempt cap 48. Both bind target 8192, quantum
exponent -20, unit lengths, viscosity 1, Cox--Matthews, case
`e1236f...68f7e`, absolute tolerances `[1e-5,1e-4]`, and relative tolerances
`[1e-5,1e-5]`. The sole admitted evolution difference is integration-force
dimensions M384 on the left and M512 on the right.

`left-n384-m384-clock0512.json` preserves the published M384 source, complete
identity, exact profile field, backend, execution description, state hashes,
clock header, guard, and copied frozen plan. The right template preserves the
deployed M512 source `326eeb5...db72`, complete published binary identity,
exact profile field, backend, execution description, guard, and copied frozen
plan. Its pending strings cannot pass the adapter's SHA-256 validation. After
publication, materialize a new `right-n384-m512-clock0512.json`; do not edit
away or present the template as executed evidence.

The exact adapter admission is 2,733,113,344 bytes: two 1,366,032,384-byte
decoded coefficient states plus the fixed 1 MiB overhead. Snapshot framing is
checked separately at exact file length. The comparison is FFT-free: it
decodes the existing coefficient files, verifies their framing, hashes,
finite/Hermitian/Nyquist constraints, and computes weighted coefficient norms
and band partitions. A fresh Sulaco memory/process admission record remains
required before any root-approved invocation.

The eventual report must retain the raw adapter output and its hash. It will
report the absolute difference from `full`, each total relative difference as
`full / fine_absolute`, and the L2, H1, and vorticity multiples of the frozen
force allocations. Those allocations are the pilot relative targets
`[1e-4,1e-3,1e-3]` times the frozen force fraction 0.15, namely
`[1.5e-5,1.5e-4,1.5e-4]`. Since both retained grids are N384, `common` must
equal `full` and `newly_resolved` must be exact zero; this band identity is a
same-grid accounting fact, not a finest-grid claim. `report-template.json`
records the formulas and also preserves the M384-to-M512 force-grid storage
context.

Before use, root must review the published M512 record against the exact
identity and clock requirements in `review-provenance.json`, review the final
materialized pair, and approve one exact command. The output remains a prefix
force-resolution diagnostic with acceptance `not_assessed` and zero accepted
windows. It does not qualify force sufficiency, the endpoint, or a PDE window.
