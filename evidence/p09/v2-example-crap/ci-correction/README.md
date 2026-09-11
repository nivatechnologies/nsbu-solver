# Rust example CRAP correction

Hosted Rust run `34603432256` at source revision
`109f9cfa8dd054c0814506850c19b5eeeb90a58c` passed its coverage tests but
failed per-function CRAP because `v2_node_binding`'s display function and
closure, and `v2_regional_tracking`'s `main`, had no observed coverage.

The correction does not replay either three-clock diagnostic scenario. It
extracts the two output contracts from their existing orchestration: node
labels distinguish missing, bitwise-equal, and different retained nodes;
regional rendering distinguishes measured errors from no samples. Focused
example unit tests cover those public-output alternatives. The remaining
orchestration functions are split into small sequential steps, keeping their
unobserved paths below the CRAP threshold without changing numerical inputs,
force arithmetic, reports, or conclusions.

The compressed focused coverage, RCA, and CRAP reports are source-bound by the
summary. They are not a replacement for the whole-workspace coverage gate.
No historical hosted artifact or raw study evidence was modified.
