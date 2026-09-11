# Production physical-reduction arithmetic evidence

This P09 increment passes 385 Rust tests/allocation probes and 220 Python tests
across 294 Rust and 98 Python/stub files. Rust executable line/branch coverage is
98.69%/90.11%; Python is 99.82%/98.82%. Every required complexity, Halstead, file
size, CRAP and typing gate passes. Frozen/bootstrap checks, Rustdoc and fresh
packaging pass. Source-matched hosted [Rust](hosted-rust.json) and [Python](hosted-python.json) checks pass.

The [public guide](../../../docs/REDUCTION_ARITHMETIC.md) explains the exact-word
formats, complete commands, resources and numerical interpretation. Actual N=4
and N=12 CM/HO component words come from the unchanged derived-field study.
Both production tensor-reduction entry paths are compared with an independent
80/120-digit sum-of-squares oracle. Rounded physical magnitudes remain separate
inputs, exposing magnitude-construction and accumulation effects. All 144
per-statistic precision-separation checks pass. These are smooth arithmetic
measurements, not concentrating-grid floors or window tolerances.

[summary.json](summary.json) records both complete quality profiles, numerical
results, source and input identities, SOLID review and limitations. Every raw
report, input/output packet and execution script is indexed in
[artifact-sha256.json](artifact-sha256.json). The complete maintained scope is
[source-sha256.json](source-sha256.json). Both independent clean-source packet
exports, Rust outputs and full numerical study reports reproduce byte-for-byte.

The initial closed-form test's excessive exact-equality requirement at 80 digits
is preserved in the failed focused log. Equivalent MP expressions differ in their
last rounding bit; the corrected independent control uses a 1e-79 absolute
tolerance. Final focused and complete tests pass.

P08/P09 remain incomplete. Full residual/balance/regional/location arithmetic,
reference/force/transfer/benchmark/artifact integration and current concentrating
grid qualification remain. No concentrating PDE window is accepted.

Final formatting removes one empty EOF line. The measured initial source inventory
is preserved; the Python AST and every executable source location are identical.
Both final source directories rerun all eleven new tests, and strict typing passes.
The source inventory and clean proof bind the final bytes.
