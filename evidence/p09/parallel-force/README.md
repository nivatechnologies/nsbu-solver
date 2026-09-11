# Bounded persistent parallel force evidence

All local gates pass: 405 Rust tests/allocation probes across 315 Rust files,
98.67% executable line and 89.86% instrumented branch coverage,
and every required complexity, Halstead, CRAP and file-size threshold. The 98
unchanged Python/stub files retain the complete 220-test profile. Bootstrap tests
(41), frozen mathematics, strict linting, Rustdoc and fresh packaging pass.
Hosted verification for this increment is pending.

The [public guide](../../../docs/PARALLEL_FORCE.md) explains plane ownership,
finite jobs, construction/stack budgets and failure/shutdown. Complete anisotropic
spectra and root work match the serial evaluator for 1/2/5/12 workers, including
rest and repeated/backward clocks. Each worker's injected numerical failure and
a programming panic retain prior coefficients, drain submitted completions and
terminate the pool. Idle/submitted shutdown joins and releases shared storage.
The isolated allocator checks the first call as well as subsequent/refused calls.

Two complete N16/N32 profiles, from the development tree and clean export, retain
all 88 serial/parallel coefficient-hash and work comparisons. Clean preflights
match byte-for-byte; all numerical output matches after removing elapsed seconds.
The development N32/32-worker median speed ratio is 12.44 against the already
optimized serial evaluator. Timing is a small-grid observation on a shared host;
these profiles do not evolve a PDE trajectory.

[summary.json](summary.json) records execution scope, quality, resource and timing
measurements. [source-sha256.json](source-sha256.json) binds all 413 maintained
source files; [artifact-sha256.json](artifact-sha256.json) inventories compressed
raw reports and replay commands. The focused coverage command initially used an
unsupported option combination, then omitted the harnessless allocation file
from reporting. Its retained raw log records those tooling failures; the corrected
complete-scope report and full workspace run pass without source exclusions.

P08/P09 remain incomplete. Worker stacks/native allowances are planning values for
the tested execution profile, not universal operating-system memory bounds.
Programming-panic recovery is outside the normal allocation-free numerical
contract. Accepted concentrating PDE windows remain zero.
