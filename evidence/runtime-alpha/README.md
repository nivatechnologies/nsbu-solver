# Runtime alpha validation

The bounded exact-v2 runtime alpha passes its local R01–R04 release exits.
[summary.json](summary.json) records measurements and limitations. The release
workflow additionally requires both hosted workflows to succeed for the exact
source commit before publishing a prerelease.

## Demonstrated behavior

- CM and HO independently evolve the default N4/M4 exact-v2 case from zero through
  32 committed steps to t = 1/256. Full-band coefficients agree with independent
  high-precision direct-DFT fixtures within an absolute 5e-13 tolerance.
- Serial and persistent-worker force paths preserve tested coefficient words,
  history and charged work. Diagnostics use their own doubled grids and force
  provider; the analytical reference never initializes or resets state.
- Complete same-profile runtime checkpoint/resume preserves the next step and
  canonical archive in both methods. Corrupt, mismatched and over-budget inputs
  are refused. External continuation remains explicitly unverified.
- Actual rejected/refused attempts preserve the last committed physical state
  and retain charged work. Admission and first/repeated committed steps pass
  allocation instrumentation. The measured serial owner heap is 193,600 bytes
  under its 260,120-byte reservation.
- A separate clean checkout packages all three crates, installs the CLI, runs
  default CM/HO and saves HO after 16 steps before resuming to 32. The resumed
  JSON equals uninterrupted output except for its external origin.

## Checks and scope

All 425 Rust harness tests and nine isolated allocation executables pass, along
with one compiled Rustdoc example, strict formatting/Clippy/Rustdoc and packaging.
Measured line coverage is 98.56% and instrumented branch coverage is 88.79%.
Maxima: cyclomatic 21, cognitive 18, function/file Halstead 75.8956, physical file
length 477, and CRAP 24.33594. [Quality policy](../../docs/QUALITY.md) explains
thresholds and informational duplication/dead-code/mutation findings.

The maintained source inventory contains 332 Rust and 98 Python/stub files.
The latter are byte-identical to the retained complete 220-test Python profile;
41 bootstrap tests and the preserved mathematics were rerun for this alpha.
[Python reuse](python-source-reuse.json) makes that distinction explicit.
The clean checkout reused local Git objects and existing dependency caches;
it does not claim a fresh online download of every dependency.

[Source hashes](source-sha256.json) bind the measured code. The
[artifact inventory](artifact-sha256.json) records compressed and uncompressed
SHA-256 values for raw logs, coverage and metric outputs. Gzip files use a zero
mtime and can be inspected with `gzip -dc FILE.gz`. The complete execution
scripts are archived with the reports; their absolute paths describe this
execution environment and must be adapted for another checkout. Final docs and
CI edits do not change the measured Rust/Python source inventory.

The [runtime reports](runtime-summary.json) and
[clean installed reports](clean-ho.json) are actual machine output. The two
`base-hosted-*` records concern the pre-alpha base only; they are not substituted
for the release workflow's exact-commit CI requirement.

## Reproduce

Use the pinned Rust/Python/tool versions described in
[installation](../../docs/INSTALL.md), [quality](../../docs/QUALITY.md) and
[the Rust workflow](../../.github/workflows/rust.yml). Run the repository checks,
bootstrap tests and design runner first. The Rust workflow gives the complete
instrumented coverage command, per-function metric gates, clean packaging and
installed checkpoint/resume checks. The [runtime guide](../../docs/RUNTIME_ALPHA.md)
provides both CLI and public-library construction examples. Fresh evidence belongs
under ignored `work/`; never overwrite frozen design reports or expected hashes.

There are **zero accepted concentrating PDE windows**. N4 agreement does not
resolve space or force sampling, and P08/P09/P10 qualification is incomplete.
Checkpoint word equality is demonstrated within the tested build/environment;
compiler and processor identity are not authenticated by the alpha container.
These results establish a usable diagnostic runtime, not a validated
concentrating PDE trajectory or a source-construction result.
