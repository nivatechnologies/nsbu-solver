# Optional reduced sampled force provider

The optional `provider::reduced::ReducedV2Force` implements the bounded public
`PrescribedForce` interface using the separately checked reduced-coordinate
algebra. It does not change the released runtime's provider selection.
[The summary](summary.json) records the source inventory, commands, measurements
and limitations. P08/P09/P10 remain incomplete; zero PDE windows are accepted.

All 17 focused tests/probes pass: eight library tests, six integration tests,
one example and two allocation executables. These include four new axial-cache
tests and three new provider integration tests. Every retained coefficient on
three sample layouts agrees with independent uncached direct DFT and the
original Cartesian provider within the 5e-12 scaled L1 threshold. The largest
differences are 5.10e-14 and 6.70e-14 respectively. The independent 18-row N4
80/120-digit force fixture passes, including a nonzero longitudinal raw-force
mode; the provider does not project the force before the numerical RHS uses it.
Existing 84 exact-word pointwise fixtures remain passing.

The cache binds each canonical axial coordinate and all elapsed/remaining time
and conversion-error words. Active points refuse missing or mismatched roots;
there is no hidden uncharged fallback. Repeated, backward and rest requests have
the expected work counts, and repeated clocks reproduce coefficient words.
Validation failures preserve output words and last-success diagnostics. A later
exceptional transform failure can partially write caller scratch, so integrated
state still relies on the outer numerical transaction.

N4 with M=[6,8,12] uses 30,560 measured constructor heap bytes against a 32,088-byte
reservation. Admission/refusal and all five successful plus three refused
post-construction calls allocate, reallocate and deallocate zero times.
Caller output storage is separate from the provider reservation.

Focused quality covers all seven added Rust files and the changed pointwise
assembly file: 710/719 executable lines (98.75%), 58/64 branches (90.625%) and
maximum CRAP19.125. Whole-workspace maxima remain CC21, cognitive18,
function/file Halstead75.8956 and 477 physical file lines. Formatting, strict
Clippy and Rustdoc pass; the type-escape scan has zero code findings. The initial
example CRAP failure is retained in the evidence; separating resource admission
from execution resolved it without changing a threshold. Full source-matched
hosted quality gates remain required before daily publication.

The documented release profile preflights N16/M24 and four nonmonotone exact
requests per provider. Complete sample construction, cache, FFT and transfer
take a median 6.48 times less elapsed time with the optional reduced provider in
this one shared-host run. Maximum scaled coefficient difference is 2.35e-14.
Both implementations report identical work units and three transforms per call.
The joint reservation is 9,815,792 bytes, including an 8 MiB declared stack
allowance; stack high-water usage is not measured. Timings are empirical and
exclude PDE integration. No runtime speed, force-grid convergence or universal
arithmetic error bound is established.

`source-sha256.json` identifies the maintained source used for these checks.
`artifact-sha256.json` verifies archived raw reports and their uncompressed
bytes. The recorded scripts preserve historical checkout paths; substitute the
checkout path when reproducing the commands. Python source is unchanged across
98 maintained files; the existing 220-test hosted result is retained separately.
Mutation and duplication sweeps are informational and were not rerun here.
