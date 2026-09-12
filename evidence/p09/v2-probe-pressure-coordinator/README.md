# Source-bound pressure coordinator verification

This evidence binds commit `606e7e4a1445007447bbb43393c32324a50d1681`.

The selected LLVM run passed the coordinator (three tests), allocator executable,
and export tests (three tests). It measures the v2-only reconstructed-pressure
payload, immutable seven-event routing comparison, joint-cap refusal, and zero
steady-event allocation. The raw production CRAP invocation initially reported
42 because an existing in-source JSON unit had no mapping in the selected
integration targets. That raw report is retained. The same LLVM profile then
ran that exact existing unit without cleaning; the combined report passes with
production CRAP 22.5. Changed test maxima are export 6, coordinator 9, and
allocation 3.

RCA reports CC21, cognitive21, and all-node Halstead difficulty 75.929515.
Raw tracked Rust file length is at most 477. Workspace all-target Clippy passes.

Schema v2 always serializes reconstructed pressure and its admitted work ledger;
schema v1 retains its old shape. The additional consumer increases joint
admission storage. Every report remains `UnqualifiedDiagnostic`; this evidence
makes no PDE-window or numerical qualification claim.
