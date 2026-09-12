# Partial exact-v2 review-profile evidence

This source-bound artifact records the admission-only exact-v2 review profile at source commit `1853eeaf2d7df5f8c75b175ac344a8c09cc6d083`, based on `31e99a17f97aec2ee18b26c67f8be88a0e931088`.

The profile admits three nested time manifests, all four existing probe-plan reconstruction geometries, 88 typed scalar semantics, nine mandatory missing groups, a 616-row schedule, and caller-owned generic verification policies. It creates no observations and has the fixed status `PartialUnpopulatedDiagnostic`. Its profile identity and canonical bytes bind both source family and probe-plan identities. The geometry is admitted plan geometry; this artifact does not claim that a numerical owner published those histories.

The focused tests use unit-valued budgets only as API controls. They are not physical tolerances. No scalar measurements, convergence finding, accepted concentrating window, pressure reference/gauge, peak rule, nominal/collar coverage, complete global qualification, or mean-momentum result is claimed.

`raw/semantics_oracle.py` independently constructs the 88 descriptors and nine gaps and reproduces `e78576813a6bc3c0de69967b679a924707a026ca9649cd988f58d5086377692e`. The Rust identity regression independently hashes the emitted generic canonical tail, then applies `SHA256(tag || family || probe || generic_digest)` and checks the frozen profile digest. It also proves an equal-manifest altered family has distinct bytes/identity and refuses mismatched expected source identities.

Focused source coverage for the new `review_profile` production module is 212/215 lines (98.6047%) and 23/24 branches (95.8333%). Production CRAP covers 40 maintained functions and peaks at 18.1181. Cargo LLVM coverage does not report integration-test source mappings in this selected invocation; every changed test function was therefore conservatively evaluated at zero coverage, with maximum CRAP 12. Whole-tree RCA at the source commit reports maximum function CC 21, cognitive 21, function Halstead difficulty 60, all-node Halstead difficulty 75.929515, and maximum tracked Rust file length 477.

Use `raw/reproduce.sh` from the repository root. Raw coverage, metrics, CRAP, test, Clippy, Rustdoc, formatting and oracle outputs are gzip-compressed beside it. `source-sha256.json` binds every changed source file; `artifact-sha256.json` binds the review artifacts.
