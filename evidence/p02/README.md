# P02 numerical foundations

Validated anisotropic periodic domains use positive finite lengths and viscosity,
four-multiple retained grids and even padded layouts. Tests exhaust small-grid
storage indices, signed modes, negative-last-axis conjugate reconstruction,
strict Nyquist refusal and half-spectrum Parseval weights. Spectrum validation
checks finite coefficients and zero-plane conjugacy without modifying the input.

Exact clocks store elapsed and remaining u128 counts separately. Every quarter
stage is formed from the committed clock with checked arithmetic. Tests cover
zero/misaligned intervals, target exhaustion, invalid restored clocks, overflow,
extreme quantum exponents and the binary64 sum counterexample. Provider conversion
is deliberately a later obligation; no rounded subtraction is advertised as exact.
Plan and numerical epochs are separate checked identifiers.

Resource tests reproduce all four reviewed cubic base reservations and check
anisotropic class totals, the exact byte-cap boundary and overflow before grid
allocation. FFT, force, diagnostics and overhead are explicit additional classes.
This ledger validates caller declarations; it is not an admission of an unknown
provider's memory usage. State allocation requires an approved reservation and
an exact from-rest clock. Six component buffers in two independent states have
distinct pointers. Allocation refusal is tested without attempting enormous storage.

All 19 Rust tests pass, including CLI tests. Complete executable line coverage is
644/644 and instrumented branch coverage 78/78. Maximum function cyclomatic
complexity is 11, cognitive complexity 8, and file/function Halstead difficulty
54.54. CRAP is at most 11. Duplication detection reports zero. The maintained
source/test maximum is recorded exactly in [summary.json](summary.json), below 500.
Mutation testing generated 157 mutants: 144 caught, 13 unviable, zero missed or
timed out. Unviable replacements have explicit build failures and are not kills.
Full outcomes remain available in [mutation-outcomes.json](mutation-outcomes.json).

SOLID review: geometry, layout mapping, exact clocks, generation counters, storage
planning, input validation and buffer ownership have separate modules. Public API
contracts have integration tests. Mutable trajectory storage is neither Clone
nor shared; reference providers cannot initialize the state. No force, scheduler,
I/O or FFT dependency enters the clock. Public APIs are intended numerical
foundation entry points, covered through public tests; private functions are
exercised. Compiler/Clippy and manual review find no dead code, redundant algorithm
or dynamic Any escape. Independent oracle logic is retained in tests.

LLVM's instrumented branch scope and inactive duplicate instantiations are
reported honestly in the full coverage report; region and instantiation coverage
are separate measurements, not claimed as 100%. Package dry-run passes. Local
paths in published raw reports are normalized as in P01. Hosted CI must pass
before this package is marked complete. No Rust PDE integration is claimed.
