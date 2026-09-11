# Exact-v2 full diagnostic event export

`v2_experiment::diagnostic::export` writes the complete seven-event fixed startup
diagnostic as one schema-versioned JSON document. It preserves the raw full,
common, and newly resolved spectral norms; all five physical and pressure pair
findings by quantity; global and five-class regional reference findings; node
provenance and coefficient equality; and probe and residual findings. Clocks,
steps, epochs, accepted-step counters, resource sizes, and work counters are
decimal strings. Small array indices, dimensions, component counts, and the
schema version remain JSON numbers.
No cross-quantity maximum or other numerical reduction is introduced.

`DiagnosticExportPlan::new` binds the owned `StartupProfile` arrays to the
admitted diagnostic family and probe identities. Its checked reservation uses
an auditable per-event ceiling of 4,096 binary64 fields at 32 bytes, 1,024
quoted counters at 41 bytes, 512 complete clocks at 160 bytes, 64 hashes at 66
bytes, and 256 KiB for fixed keys and punctuation, plus 64 KiB document
overhead. These inventories exceed every fixed accepted and residual variant;
the validation pass also refuses an underestimated bound. The work bound
covers two complete byte visits and at most one writer call per visited byte
plus a terminal failed call. The exporter
uses constant-size formatting state and borrows the retained events; caller
storage for a file, buffer, or stream remains outside the exporter.

The cardinality review counts 15 band comparisons in an accepted event and 15
in a residual event; each has 15 binary64 fields. An accepted event additionally
has 20 physical and 10 pressure `LocalError` records, 60 physical extrema, and
24 regional quantities with one global plus at most six measured regional
`LocalError` records. This remains below 1,500 binary64 fields. A residual event
adds six norm/domain records and fixed reconstruction geometry and remains below
512. Both are below 128 complete clocks and 16 hashes. The published ceilings
retain more than twofold cardinality margin. Fewer than 4,096 keyed values with
keys no longer than 32 bytes fit below half the fixed-text allowance; punctuation
and fixed variant labels use the remaining half.

`write_json` first walks the complete report into a counting sink. It refuses a
wrong clock, identity, schedule, non-finite value, or underestimated bound before
touching the caller's writer. It then emits directly to the writer. I/O is not
transactional: an error reports the completed validation bytes and the exact
number of prefix bytes already accepted by the writer. A caller that needs
atomic file publication must write to a private temporary file and rename it.
The byte/call reservation bounds exporter-controlled serialization work. It
cannot bound wall time inside an arbitrary caller-supplied blocking `Write`
implementation. A prevalidation refusal occurs before emission and therefore
has no successful-work report; an I/O refusal retains validation bytes and
successfully accepted prefix bytes.

Every document and event says `UnqualifiedDiagnostic`. `NotScheduledAt...`,
`NoSamples`, `MissingRetainedNode`, and all ten missing evidence channels are
explicit variants rather than zero values. The five regional classifier outputs
are exclusive for each sampled point. Their counts do not prove nominal-region
coverage or volume enclosures.

This export advances P08 artifact handling for the current fixed binary64,
current-grid diagnostic. It supplies neither a complete artifact/provenance
bundle nor continuum, reference-precision, pressure-gauge, convergence, or PDE
window qualification.

Independent decoding tests use the dev-only `serde_json` 1.0.145
`float_roundtrip` feature. Serde JSON is dual licensed MIT OR Apache-2.0 and is
not linked into the production library exporter.
