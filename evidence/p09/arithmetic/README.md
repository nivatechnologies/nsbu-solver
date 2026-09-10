# Independent same-grid smooth arithmetic

The [execution summary](summary.json) and [source inventory](source-sha256.json)
bind this increment to 229 Rust files and 70 Python source/stub files. All 309
Rust harness tests and four allocation executables pass. The complete Python
suite passes 184 tests. Required quality gates pass locally; source-matched
hosted verification is pending.

| Measurement | Rust | Python |
| --- | --- | --- |
| Executable lines | 19,443/19,656 (98.92%) | 4,187/4,196 (99.79%) |
| Instrumented branches | 1,416/1,552 (91.24%) | 788/796 (98.99%) |
| Maximum cyclomatic complexity | 21 | 16 |
| Maximum cognitive complexity | 16 | 19 |
| Maximum function/file Halstead difficulty | 75.8956 | 13.8261 |
| Maximum physical source-file lines | 385 | 243 |
| Maximum per-function CRAP | 24.33594 | 16 |

Strict Python typing reports zero errors, warnings or notes. Format, Clippy,
Rustdoc, fresh-target workspace packaging, fresh-source installation and installed
CLI preflight/HO execution pass. Repository integrity, all 41 bootstrap tests,
and original mathematical verification pass. These remain distinct from PDE
qualification.

The [public walkthrough](../../../docs/ARITHMETIC_STUDY.md) reproduces the complete
study. Sixteen independent reference trajectories cover N=4 and N=12, CM and HO,
80 and 120 digits, and independently evaluated versus fixed-bit prescribed force.
Each starts at exact rest, executes eight full/two-half proposals, and advances
through the sixteen fine updates. Analytical fields never replace integrated
state. CM executes 96 RHS calls; HO executes 120.

| Grid | Method | Maximum Rust versus fixed-input 120-digit coefficient difference |
| --- | --- | --- |
| 4 | CM | 4.80603745e-18 |
| 4 | HO | 2.29520772e-18 |
| 12 | CM | 4.80603742e-18 |
| 12 | HO | 2.29520764e-18 |

The separate trajectory effect of binary64 force evaluation is about 3.18213e-19
per coefficient. Across all measured coefficient/L2/H1 comparisons, the largest
observed ratio of the 80-to-120-digit change to the corresponding binary64
discrepancy is 2.106e-64. These are observed numerical separations, not roundoff
enclosures or a frozen concentrating-window tolerance policy.

The four compressed comparison reports retain **every signed complex coefficient
difference**, together with complete-band norms, projected force differences over
all 33 exact clocks, input hashes and unmodified reality defects. The `inputs/`
directory retains exact Rust state/force words; `trajectories/` retains every
80/120-digit full-band state. The [raw artifact inventory](artifact-sha256.json)
records both compressed and original byte counts and SHA-256 hashes. Standard
Python `gzip` can read the deterministic `.json.gz` payloads.

The [fresh-source proof](clean-source-proof.json) confirms that all eight N=4
reference reports and both comparison reports reproduce byte-for-byte from a
public source export. The export also builds the Rust producers and CLI; its
N=12 state exports supply the finest-grid comparisons. The
[trajectory launch snapshot](trajectory-launch-source.json) matches all numerical
source modules used by the long runs. Exploratory scripts under `work/` are not
the source of these maintained-code results.

Negative controls reject wrong grid/method/step/precision metadata, altered force
identity, missing or reordered modes, nonfinite inputs, nonzero Nyquist entries,
malformed complex values, ambiguous JSON and excessive input size/depth. A high
mode and an asymmetric conjugate pair remain visible in the comparison instead
of being cropped or repaired. Fixed inputs remain explicitly labeled fixtures;
their hashes and internally consistent metadata cannot authenticate physical
origin or promote an imported report to a qualified trajectory.

SOLID review separates numerical admission/evolution, immutable prescribed data,
strict report decoding, comparison and output serialization. The state exporter
initially reached CRAP 34.125. Separating evolution from read-only serialization
resolved the [preserved finding](precorrection-export-crap.json); the final full
run passes. Python's nine informational dead-code findings are discovered test
classes or schema fields. Duplication and mutation findings remain informational
under the revised policy; no new mutation sweep was run. The compressed raw
coverage, complexity and CRAP reports retain the measured evidence.

**P08/P09 remain incomplete and accepted concentrating PDE windows remain zero.**
This increment measures smooth velocity arithmetic at its actual finest grid.
Complete observable/time inventories, frozen tolerances and artifact semantics,
pressure/regional diagnostics, sampling/quadrature refinements and concentrating
integration/qualification remain separate obligations. N=12 arithmetic evidence
does not establish roundoff control on a larger concentrating trajectory.
