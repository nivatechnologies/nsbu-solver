# Exact-v2 physical consumer evidence

This bounded evidence package covers the v2 physical refinement consumer. The
focused gate selects seven maintained Rust files: the physical implementation
and plan, the v2 family plan, three physical tests/oracle files, and the release
example. It does not claim whole-workspace coverage or scientific qualification.

The focused suite has three physical harness tests, one isolated allocator
executable, and one release example; the retained family admission/identity
tests are also run by the gate. The physical test uses six independent from-rest
branches at N=[4,8,12], fixed force M=12, steps [64,32,16], exponent -20,
target 8192, and clocks [0,64,128]. It checks all four quantities and five
actual pair mappings with a direct full-grid Fourier oracle. The allocator log
records zero allocations during planning and steady measurements.

Focused LLVM totals are 702/718 executable lines (97.7716%) and 49/58 branches
(84.4828%); focused maximum CRAP is 19.6132. These totals are derived from the
seven selected files in `raw/v2-physical-focused-coverage.json.gz`; LLVM totals
for dependencies and unselected files are retained but are outside this scope.
Fresh whole-source maxima are CC 21, cognitive 18, Halstead difficulty 75.8955,
and maximum physical file length 477 lines. The raw complexity report is
`raw/v2-physical-metrics.json.gz`; file length is measured on the inventoried sources.
The complete inventory is 355 Rust files, 97 Python files, and one `.pyi` stub.

The release example reaches elapsed ticks 0, 64, and 128 and ends with
`accepted_pde_windows=0`; its final physical diagnostics are retained verbatim.
The independent signed Fourier oracle checks all five pair mappings across all
four quantities on the fixed full sample grid. Both RMS and peak differences
must satisfy `abs(measured - oracle) < 2e-11 * (quantity_floor + oracle)`.
The tests also check report
identity and floors, exact state words before and after diagnostics, stale
same-clock, wrong-identity, terminated-family and exhausted allowance refusals,
and charged attempt transactions. The physical reservation charges 1,350 scalar
transforms and 18,535,872 weighted visits for three attempts.
The profile endpoint is the startup time 1/8192, before the first concentrating
endpoint 1/256. The Hessian spatial differences increase on these coarse grids;
no spatial convergence is claimed. Full source-matched hosted gates remain
pending. P08/P09/P10 remain incomplete and accepted PDE windows remain zero.
Raw logs and reports are gzip compressed under `raw/`. `artifact-sha256.json` records
their hashes; `raw-sha256-uncompressed.txt` records uncompressed raw hashes;
`source-sha256.json` records the complete source scope and
`selected_focus_files` in `summary.json` records the seven focused files.

The archived gate script records the exact focused commands and historical
checkout path. Substitute that path for a new checkout. The packaging script is
retained as historical evidence; measurements in the summary were independently
recomputed from the archived JSON and source inventory. An unnecessary second
focused run was interrupted after the first complete run passed; the retained
passing reports and source hashes are from the original completed run.
