# Independent reconstructed balance quadrature evidence

This P08 increment passes 391 Rust tests/allocation probes across 302 Rust files:
98.68% executable line and 89.93% instrumented branch coverage.
All complexity, Halstead, file-size and CRAP gates pass. The 98 unchanged
Python/stub files retain their complete 220-test profile; 41 bootstrap tests and
frozen mathematical checks rerun successfully. Both source-matched hosted workflows pass at commit `734efb6`: Rust
[34551437057](https://github.com/nivatechnologies/nsbu-solver/actions/runs/34551437057)
and Python/repository
[34551437049](https://github.com/nivatechnologies/nsbu-solver/actions/runs/34551437049).

The [public workflow](../../../docs/BALANCE_QUADRATURE.md) retains sixty complete
balance samples, every original accepted-node provenance record and eighteen
integrals on three strictly nested quadrature schedules. State/interpolant digests
remain unchanged by observation. Invalid geometry, finite work/storage bounds,
all six child-failure positions and later staged-quadrature failure are tested.
Allocation instrumentation passes, and a clean source export reproduces every
resource declaration and numerical report byte-for-byte.

The initial test suite passed, but one aggregation function had CRAP 30 because
an internal check compared two views of one immutable private probe record. The
[initial report](initial-rust-crap.json.gz) remains preserved; adding all six real
child-failure controls did not cover that impossible condition. The redundant
comparison was removed after tracing ownership, retaining the original
owner/manifest validation and complete failure controls. The full coverage suite
was rerun after this simplification.

[summary.json](summary.json) records quality maxima, source scope, measured work
and limitations. [source-sha256.json](source-sha256.json) binds every maintained
source/test file. [artifact-sha256.json](artifact-sha256.json) indexes compressed
raw reports, including the complete public output and fresh coverage.

P08/P09 remain incomplete. These short smooth quadrature measurements do not
bound reconstruction error or qualify a concentrating PDE trajectory. Accepted
concentrating windows remain zero.
