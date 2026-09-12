# Exact-v2 startup pressure-reference evidence

This source-bound artifact records the startup-only analytical pressure consumer at source commit `d963f88674b63b7cc01fc22c065a3f8924fb4961`, based on integration commit `78aaf2a0af908a5c79a38b1d52cb61bdda17e576`.

The consumer uses the existing original-force pressure construction on the fixed M24 doubled grid, samples pressure and its three-component gradient on a 24-cubed lattice, and compares all six actual accepted branches with `fields::reference`. One imported global unit-cube mean is subtracted at every point; gradients are unchanged. Publication occurs only after all branches finish.

The exact-byte allowlist is limited to startup clocks 0, 64 and 128. Each authoritative JSON retains ten means: joint 8/16/32 and crossed 16x32/32x16 Simpson profiles at both 80 and 120 digits. Rust hashes the actual imported bytes, retains every raw decimal/profile/change, exposes all ten binary64 roundings, and tests the generated projection against independently parsed JSON. These are empirical refinements, not certified bounds.

Focused source-matched tests passed. At clock 64 the finest CM branch pressure/gradient RMS errors were `2.3396815209674412e-11` and `1.5156023160597217e-9`; at clock 128 they were `6.083804293434028e-5` and `3.8093010562817026e-3`. These are diagnostic values without accepted budgets or convergence claims. Construction allocated 9,079,296 bytes against 9,147,992 admitted consumer bytes; joint family+consumer storage was 33,338,416 bytes; one rest report allocated zero steady bytes. The per-attempt ledger charged 1,783,296 provider work units and 81 transforms (provider plus 54 conservative plus 24 derivative transforms).

Focused coverage of the new pressure-reference production module is 396/411 lines (96.3504%) and 21/26 branches (80.7692%). Production CRAP covers 65 changed functions and peaks at 14. Changed integration-test functions are absent from the selected LLVM report and were conservatively evaluated at zero coverage; maximum CRAP is 20. Whole-source RCA maxima are CC 21, cognitive 21, function Halstead difficulty 60, all-node difficulty 75.929515, and file length 477.

The existing two pressure-kernel unit regressions passed at `f9660c0`; the shared pressure source blob is identical at final source `d963f88`. They include independent serial-force/pressure bit comparisons. Strict focused Clippy, formatting and exact workspace Rustdoc passed at final source.

This increment does not admit the available clock-4096 artifact, define pressure budgets, close general pressure-reference/gauge requirements, establish quadrature enclosure, or qualify a concentrating window. P09 and P10 remain incomplete.
