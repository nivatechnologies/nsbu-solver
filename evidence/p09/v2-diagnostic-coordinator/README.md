# Exact-v2 unqualified diagnostic coordinator

This increment composes two independently evolved six-branch families and the
existing physical, pressure, regional analytical-reference, residual and exact
accepted-node consumers. The complete manifest is `[0,7,63,64,95,127,128]`.
Accepted clocks run every accepted-state consumer; the four other clocks are
independently admitted genuine off-stage residuals. Each event explicitly marks
the other path `NotScheduled`, carries `UnqualifiedDiagnostic`, and retains ten
missing channels. No acceptance threshold or PDE success claim is introduced.

Three integration tests pass admission/refusal, terminal publication and full
seven-event wiring. At accepted clocks 0, 64 and 128, separately admitted
physical, pressure and regional-reference consumers reproduce every raw
coordinator finding on the same read-only ordinary states. Exact accepted-node
findings are bitwise equal on all six branches. The nontrivial off-stage signal
reaches L2 `5.1013585659162156e-2` at tick 127; it is an observation, not a
bound or convergence finding.

Planning allocates nothing. Driver construction uses 58,981,808 bytes within the
69,433,968-byte joint reservation. The reservation counts both families once,
every incremental consumer, seven retained events and two transient event
copies. Caller-retained copies are outside the owner contract. The complete
seven-event allocator execution performs zero allocations. Actual consumer
charges remain inspectable after success or failure.

The release example and its `--dry-run` path pass. The dry run stops before
`DiagnosticDriver` allocation and prints storage, work and missing channels.
The actual run retains all seven events and ends with `accepted_pde_windows=0`.

Nightly coverage over the three coordinator production files is 365/390 lines
(93.59%) and 24/30 branches (80.0%). Maximum production CRAP is 16.0417. A
JSON-lines parse of all 425 full-tree RCA records selected the three production
files, two tests and example. Across their 70 function records, maxima are CC15,
cognitive 8 and Halstead difficulty 40.6154; the largest selected physical file
is 329 lines. Strict workspace Clippy, formatting and the exact
`RUSTDOCFLAGS='-D warnings' cargo doc --workspace --no-deps --locked` command
pass after the final source edit.

The raw full-tree RCA JSON-lines, focused RCA, LLVM coverage, CRAP and example
output are stored compressed. `source-sha256.json` identifies the exact source
commit inputs and `raw/check_v2_diagnostic.sh.gz` reproduces the gates. The
allocator was run normally rather than duplicated under LLVM instrumentation;
the instrumented integration test covers both coordinator event paths.
