# Pilot arithmetic Python-quality correction

Hosted Python run `34603432124` at source revision
`109f9cfa8dd054c0814506850c19b5eeeb90a58c` passed its tests but failed the
per-function CRAP gate. Its preserved artifact attributed the failure to the
legacy pilot's unexercised `rust_rows` subprocess boundary (CRAP 30) and the
untaken malformed-evaluator-shape guard in `tracking_values` (CRAP 26.125).
The existing orchestration test executed `run` with `rust_rows` mocked; it did
not cover that boundary.

The correction adds only fast Python tests. It mocks the completed producer
process, so Python CI has no Rust-binary dependency, and checks a valid
12-record stream and digest, wrong count, malformed record, and subprocess
failure. A controlled four-velocity evaluator response exercises the existing
42-entry shape guard. The reference evaluator and scientific calculations are
unchanged.

`coverage.json`, `python-cyclomatic.json.gz`, and `python-crap.json.gz` were
made in this correction worktree by focused coverage of both maintained pilot
and full drivers. Both files exceed 80% executable lines and branches, and the
measured maximum CRAP is 19.9814453125. The historical `raw/` evidence was not
changed.
