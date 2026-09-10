# Smooth file checkpoints and accepted reconstruction

[The execution summary](summary.json) binds this increment to all 209 maintained
Rust source, test and example files. Local checks pass 280 harness tests and four
isolated allocation executables. Coverage measures 17,457/17,619 executable lines
(99.08%) and 1,291/1,410 instrumented branches (91.56%). Maximum CC is 21, cognitive
complexity 16, function/file Halstead difficulty 75.8956, physical file length 385
and CRAP 24.33594. Compressed raw reports and execution logs are retained here.

The CLI saves balance-only smooth runs after an exact accepted-step count and
resumes their original finite configuration as `external_unverified`. It refuses
out-of-range save counts before integration, preflights file buffers, preserves
existing destinations and temporary-name collisions, and explicitly reports the
case where publication succeeded but directory durability could not be confirmed.
Deterministic reader tests reject short and growing input. A fresh source export
installs with public dependencies; 14 installed CLI cases pass, including CM/HO
save/resume reports matching uninterrupted runs except for their origin label.

The independent reconstruction observer stages actual endpoint values and a
conservative double-grid RHS. The runner publishes this proposal only after
physical and raw-history commits. Local rejection, measurement failure and
history failure cannot publish a node. Its three accepted endpoints support
off-stage Hermite probes without integrator-stage derivatives or reference
assignments. Allocation probes cover accepted-ring rotation and interpolation.

`SmoothRun` and `ReconstructedRun` select observation profiles through the same
owned integration implementation. Trusted reconstruction snapshots retain values,
derivatives, exact clocks, epochs and all spent work. Tests restore every phase
of the accepted ring and reproduce the next CM/HO transaction and interpolant
bit for bit; rejection, diagnostic exhaustion and terminal snapshots retain
their state and budgets. Complete reservations are checked before copies, and
restored attempts/interpolation allocate no heap storage.

SOLID review separates pure plan admission, ownership, numerical observation,
snapshot validation, binary I/O and JSON presentation. Initial measured CRAP
failures in CLI reading/import and snapshot validation prompted responsibility
splits and meaningful reader regressions; the final combined run passes. Clone
detection reports 27 matches (253 lines, 1.0064%), primarily fixture setup,
explicit representation layouts, allocation helpers and related assertions.
These findings are informational; independent numerical verification remains
separate. Clippy and scoped API review are the dead-code checks. No mutation
sweep was run for this increment and no universal absence claim is made.

The binary file format still accepts only balance-only `SmoothRun`; it cannot
silently discard reconstruction by accepting a `ReconstructedRun`. Binary
reconstruction persistence, complete external provenance and independent
refinement families remain under implementation. **P08/P09 remain incomplete,
with zero accepted concentrating PDE windows.** [Hosted Rust](hosted-rust.json) and [Python](hosted-python.json) checks passed
for `2f7a502b054fdb9200ead78ab54cf420f23e095f`.
