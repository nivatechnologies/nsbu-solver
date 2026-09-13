# Local modal derivative fixtures — 2026-09-13

Eight local Qwen tasks produced 312 modal derivative entries. A separate integer
oracle checked every entry; all eight passed on the first attempt in 166.94
seconds, with zero repairs and no pending manual fixture reviews. These are
artificial Fourier modes on a torus of side 2π, not PDE trajectories or accepted
concentrating windows. Scientific acceptance remains false.

`cases.json` contains checked outputs and model-output hashes. `fixtures.py`
preserves the independent checker. `state.json` contains original receipts;
`source-hashes.json` binds the runner snapshot (ec196e4). Expected answers were
withheld from the model. The Rust fixture is under
`crates/nsbu-solver/tests/fixtures/local_modal_derivatives.rs`.

Across 56 hardware samples, both GPUs averaged about 92% utilization. Historical
out-of-memory counters did not increase and no active throttling was sampled.
This short bounded workload does not establish sustained coding autonomy.
Cloud review time was unmeasured and must not be reported as zero.

The separate named-artifact smoke at runner revision dfb6ff1 passed one native
tool call and a final request with tools omitted. Reasoning was enabled for both
requests. Its independently checked nonce receipt is `native-tool-smoke.json`.

## Preserved unsuccessful trial

`failed-consumer-receipt.json` records a separate Rust consumer generation trial.
It exhausted the output limit, then the 300-second wall-clock budget. No code
was accepted or applied. This larger task class was manually paused.
The old runner incorrectly recorded zero token usage on rejected/timeout paths:
those zero values mean **unrecorded usage**, not zero inference. Server-side
cancellation was unknown at timeout; a later API observation found no running
or waiting requests. The OS exit status was not retained after interruption.

This batch does not demonstrate reliable arbitrary code generation or advance
any numerical acceptance gate. Receipts are in completion order, not case order.

## Rust consumer validation

A handwritten integration test now exercises all 312 rows through the actual
`DerivativeWorkspace` on a 16³ sample grid, with exact quarter-period phase
values and unchanged input spectra. Zero modes are counted once; nonzero waves
include their conjugate partners. The new test and five existing derivative
sampling tests passed with `cargo test -p nsbu-solver --test
derivative_modal_fixtures --test derivative_sampling`. This covers the derivative
operator, not regional classification, continuum maxima or PDE acceptance.
