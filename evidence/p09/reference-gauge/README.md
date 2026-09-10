# Independent exact-v2 global pressure gauge

All local gates pass for the complete [source inventory](source-sha256.json):
209 Python tests across 88 maintained Python/stub files, 99.80% executable line
coverage and 98.89% instrumented branch coverage. Maxima are CC16, cognitive19,
Halstead13.826087, physical-file243 and CRAP16. Strict typing reports zero errors.
All 272 Rust files remain byte-identical to the verified
[367-test/probe increment](../../p08/probe-family/README.md); no Rust numerical
code changed. Its source-matched hosted Rust and Python success is archived.

The [summary](summary.json), [artifact hashes](artifact-sha256.json) and compressed
raw reports retain the exact commands, source identities, measurements and tool
findings. Forty-one bootstrap tests, repository/frozen-input checks and the
original mathematical verification pass. A clean source export passes all eleven
new tests and reproduces the complete public ten-profile study byte-for-byte.
Hosted pressure-reference checks are pending.

## Numerical work completed

The [public pressure-reference workflow](../../../docs/PRESSURE_REFERENCE.md)
computes the frozen case's global unit-cube pressure mean. Angular integration
uses the complete support sphere, both signs of z and the exact cylindrical
Jacobian. The Gaussian radial core is integrated analytically; the complete
cutoff collar and axial direction have separate Simpson refinements. Reported
pressure subtracts the same global constant everywhere, including outside raw
support. Regional refitting and force/state mutation are absent.

Nine complete studies cover rest, startup t=1/1024, first endpoint t=1/256 and
second endpoint t=3/512. Each study uses five joint/single-direction geometries
at both 80 and 120 digits. Finer studies extend the axial/collar panels through
512. Their 90 estimates include 70 distinct profiles; overlapping profiles
reproduce every stored mean exactly. Every study exited zero and retained zero
accepted PDE windows.

At the finest recorded level, the largest joint/single-direction quadrature
change divided by the mean magnitude is approximately:

| Physical time | Relative finest quadrature change |
| --- | ---: |
| Startup 1/1024 | 1.69624e-11 |
| First endpoint 1/256 | 1.66877e-11 |
| Second endpoint 3/512 | 1.60066e-11 |
| Rest 0 | Exact zero; no defined relative ratio |

Arithmetic differences and quadrature changes remain separate in every raw
report. The original full-radius uniform Simpson pilot had a finest joint change
of about 5.16022e-5 at t=1/256, despite an 80/120 difference around 1e-82. Its
complete output and source are preserved. Exact integration of the prescribed
Gaussian core removes that narrowing core from radial quadrature. Independent
Cartesian scalar sampling converges to the revised formula in a regression test.

All profiles are admitted before quadrature. A maximum of two isolated study
processes ran concurrently, each with a 64 MiB cap and finite panel/root bounds;
observed available memory, interpreter allowances, elapsed times and measured
process resource output are retained. No unbounded adaptive quadrature or
all-node root cache is used.

## Review and remaining limits

Generic finite quadrature, immutable exact-case requests, global-gauge evaluation,
refinement assembly and command I/O have separate responsibilities. Tests cover
polynomial order, volume/Jacobian, the independent scalar formula, global-gauge
behavior, precision restoration, frozen-identity refusal, finite work/caps and
real public command failures. Static unused findings remain informational:
eight discovered test classes and three schema keys. Broad lexical type scans
found explanatory prose and an existing concrete Rust test-provider type; strict
typing and the maintained-source escape scan found no opaque type escapes.
Mutation sweeps were not rerun.

The initial uniform-quadrature full test run was deliberately terminated before
changing the radial algorithm. Its partial log is retained and is not completion
evidence. The final source's entire 209-test coverage/quality run passed.

**These are empirical reference quadratures, not rigorous enclosures or accepted
PDE tolerances.** Every future claimed probe time needs its own reference budget.
Complete regional/reference/force/production-arithmetic studies, benchmark and
external-artifact binding and concentrating window production remain. P08/P09
are incomplete; no concentrating PDE window is accepted.

Source-matched hosted [Python](hosted-python.json) and [Rust](hosted-rust.json)
checks passed. These checks do not qualify a concentrating PDE window.
