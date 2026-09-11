# Exact-v2 diagnostic force-worker evidence

This bounded performance increment routes the exact-v2 pressure and off-stage
residual consumers through the existing `RunForce` selector. Pressure preserves
its common finest doubled `M=24` sampling layout and inherits only the configured
worker count. Residuals preserve the configured integration force layout doubled
once, also `M=24` in this fixture, and use the same setting for all six children.
No force equation, sample point, transform or conservative kernel changed.

The numerical controls evolve each input from rest. For workers one and two,
pressure force coefficients and rebuilt pressure coefficients agree bit for bit
with a fresh serial `V2Force` on the same actual accepted states. All six
off-stage residual fields likewise agree bit for bit with fresh serial force and
residual assembly on the same reconstructed fields. Existing pressure and
early/late residual oracle, admission, binding, rollback and terminal tests pass.

Admission uses `RunForce::limits`, including parallel provider buffers, worker
metadata, allocation allowance and configured stacks. One-byte-short joint caps
are refused. Construction used 25,890,776/25,898,000 bytes for pressure workers
one/two within 59,001,232/87,124,080 declared joint bytes. Residual construction
used 39,373,616/39,383,200 bytes within 83,391,992/122,331,320 declared joint
bytes. Planning and steady diagnostic calls allocate nothing.

On this host, an optimized complete six-branch fixed-M24 residual measurement
took 3,115,741 microseconds with one worker and 1,868,053 microseconds with two.
These values are informational and are not an acceptance threshold or a broad
performance claim.

Focused production coverage is 750/794 executable lines (94.4584%) and 42/52
branches (80.7692%). Changed tests and allocator programs cover 627/628 lines
(99.8408%) and 31/36 branches (86.1111%). CRAP was checked across all 143 changed
maintained functions, including tests, and has maximum 16. Whole-tree analysis
has maximum function CC 21, cognitive complexity 21, all-node Halstead difficulty
75.9296 and Rust file length 477. Formatting, focused Clippy and exact workspace
Rustdoc with warnings denied pass.

This does not qualify force coverage, residual convergence, pressure gauge
convergence or a concentrating PDE window. P09 and P10 remain incomplete.
