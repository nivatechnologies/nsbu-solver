# Exact force-evaluation optimization evidence

This increment passes 398 Rust tests/allocation probes across 306 Rust files,
with 98.68% executable line and 89.99% instrumented branch coverage.
All required complexity, Halstead, file-size and CRAP gates pass. The 98 unchanged
Python/stub files retain their complete 220-test profile. Bootstrap tests (41),
frozen mathematics, strict linting, Rustdoc and fresh packaging pass. Hosted
checks for this increment are pending.

The [public guide](../../../docs/FORCE_EVALUATION.md) explains cache identity,
separate exact-clock coordinates, derivative tables, work and storage accounting.
Pointwise comparisons preserve every requested derivative and cancellation word;
a complete anisotropic spectrum independently assembled from uncached fields
matches exactly. A separate linear-search derivative oracle and the independent
Python direct-DFT force fixture pass. Twelve constructor allocations and no
allocations during repeated/refused force requests are verified.

Three interleaved baseline/combined profiles preserve all 36 complete coefficient
hashes; median measured speed ratio is 3.046 (2.576 to 3.687). The baseline numerical
sources are checked against commit 7b57fd1; both executables use the same profile
driver. All initial and interleaved raw results are retained. These measurements
are specific to small grids on this shared host, not a PDE or large-grid estimate.

A clean source export verifies all 404 maintained source hashes, passes sixteen
focused tests and the allocation executable, and reproduces complete preflight,
force hashes, work and storage output. Only measured elapsed seconds are removed
when comparing the numerical profile.

[summary.json](summary.json), [source-sha256.json](source-sha256.json) and
[artifact-sha256.json](artifact-sha256.json) bind the scope and compressed raw
reports. P08/P09 remain incomplete; accepted concentrating PDE windows remain zero.
