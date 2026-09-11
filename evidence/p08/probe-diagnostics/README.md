# Complete physical diagnostics at reconstructed probe times

All local gates pass for the complete [source inventory](source-sha256.json):
372 Rust tests/probes (366 harness tests and six allocation executables), 98.79%
executable line coverage and 90.25% instrumented branch coverage across 281 Rust
files. Maxima CC21, cognitive18, Halstead75.895523, physical-file473 and
CRAP24.335938 meet every required limit.

The 88 Python/stub files exactly match the independently verified
[209-test reference-gauge profile](../../p09/reference-gauge/README.md).
The [summary](summary.json), [artifact inventory](artifact-sha256.json) and raw
compressed reports preserve measured scope, actual output, quality findings and
source hashes. Forty-one bootstrap tests, frozen mathematical checks, formatting,
strict linting, Rustdoc and fresh-target packaging pass. Hosted checks for this
physical-probe increment completed: Python passed; Rust numerical checks passed but a prose-only lexical scan failed, as recorded below.

## Actual numerical exit evidence

The [public example](../../../docs/RECONSTRUCTED_PHYSICAL_FIELDS.md) measures all
six complete physical quantities across five actual trajectory pairs at ticks
0/7/128. All six owners evolve independently from rest on N=4/8/12 and H=64/32/16.
Full reconstructed coefficients at the physical probe clock supply the physical
samplers; later actual lookahead states remain unchanged. Complete ordered first/
second derivatives and full doubled-band mean-zero pressure/gradient are retained.

The pressure force is freshly evaluated at the probe time. At tick zero all
reported field/pressure scales are exactly zero despite the positive actual
lookahead clocks. At tick seven, directly comparing those later actual states
gives a substantially different result, and the test requires the reconstructed
comparison. Every state and interpolant coefficient digest is unchanged during
measurement. The report retains all exact accepted-node origins.

The joint reservation is 26,586,600 bytes under a 128 MiB cap, including 4,392,832
bytes for the two complete consumers/report allowance. Three successful reports
charge 1,350 physical transforms and 390 pressure transforms, 67,392 prescribed
force units, and separately reported weighted numerical visits. Invalid requests
consume a finite aggregate attempt before child work; children retain their
complete numerical charges. Pressure failure after successful physical sampling
publishes no aggregate and permanently terminates the consumer.

Allocation instrumentation verifies zero heap activity during admission,
construction within the reservation, and no allocations/deallocations/reallocations
during repeated valid and refused probes. Tests reject inadequate pressure band,
invalid floors, size/work overflow, insufficient capacity/attempts, changed exact
settings/manifests, missing/skipped probes and rejected owners.

A clean source export passes the three integration tests and the terminal-child
unit test, then reproduces all ninety public comparisons and full preflight
byte-for-byte. Integration of the independently verified Python reference work
preserves every measured Rust source, Cargo profile and runtime fixture. The
combined 369-file export again reproduces the complete Rust example and passes
all eleven new Python tests; the original 363-file clean proof is retained.

## Review and limits

Exact probe provenance, finite joint admission, existing numerical kernels and
complete report publication remain separate. Private bridges reuse physical/
pressure kernels without exposing arbitrary-field provenance or state mutation.
The kernels retain their independent direct-DFT, tensor and pressure-band fixtures.
Static unused/duplication findings are informational and their original outputs
are retained. Lexical type scans matched English documentation, not opaque types;
full comment lines are separately classified and original scans remain available.
No new mutation sweep was performed.

This increment does not complete reconstruction-error control, reference/force/
residual/arithmetic/regional/location studies, benchmark semantics, external
artifact binding or concentrating qualification. **P08/P09 remain incomplete;
accepted concentrating PDE windows remain zero.**

Hosted Python passed. Hosted Rust passed numerical tests and coverage, then failed
because its lexical type scan treated English `Any` in a documentation comment
as a code escape. [The full failed run](hosted-rust.json) and compressed failing
log are preserved. The streamed-residual increment repairs this CI classifier.
