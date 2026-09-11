# Independent optional-provider trajectories

Both CM and HO evolve separate original-provider and reduced-provider states
from exact rest on N4/M4 to the first endpoint, 1/256. Each performs 32 committed
128-tick attempts with quantum 2^-20 and target 8192. No analytical values are
assigned to either integrated state. These small-grid trajectories remain
spatially and force-sampling unresolved; zero PDE windows are accepted.

The two focused tests pass. After every commit, every retained coefficient
agrees between providers within the 5e-12 scaled component threshold. Maximum
differences are 2.915242e-16 for CM and 4.538019e-16 for HO. All three
components at the 18 explicitly listed strict-band endpoint modes pass the
independent 80/120-digit direct-DFT trajectory fixtures
at 5e-13 (the fixtures exclude Nyquist slots). Maximum scaled complex L1
fixture discrepancies are 4.816970e-16 for the original provider and
2.966118e-16 for the reduced provider.

Both resource plans and checked joint storage/work bounds precede allocation of
either provider or state. Exact clocks, acceptance counts, RHS calls, transform
counts and cumulative provider work are checked. Each branch uses 384 RHS calls
and 4,992 scalar transforms for CM, or 480 calls and 6,240 transforms for HO.
Provider work totals are 28,396 and 35,500 respectively, identical between
providers. Attempts preserve committed words until token-authorized commit. A
real reduced-provider rejection preserves committed state and consumes work.

Focused LLVM coverage is 271/271 executable lines across both new test/helper
files; maximum CRAP is 6. LLVM reports no instrumented branches in these files,
so their branch percentage is not applicable. The complete workspace branch
gate remains required in hosted CI. Whole-source static maxima are CC 21,
cognitive 18, function/file Halstead 75.8956 and 477 physical lines. Formatting and
strict Clippy pass, with zero type-escape code findings. Production code is
unchanged from `6170341`; all 98 maintained Python/stub files are unchanged.

Run the check with:

```sh
cargo test -p nsbu-benchmarks --test reduced_trajectory -- --nocapture
```

[summary.json](summary.json) records exact measurements, source and fixture
hashes, execution profile and limitations. Archived scripts preserve historical
checkout paths; replace those paths for a new checkout. Raw logs, coverage and
metrics retain their compressed and uncompressed SHA-256 inventory. Full
source-matched hosted gates remain pending for this increment.

No trajectory timing, force-grid convergence or universal arithmetic bound is
established. The default `Run`, `ForceSettings`, CLI and checkpoint paths retain
the original provider. Explicit arithmetic identity and runtime integration
remain separate work. P08/P09/P10 retain their original incomplete status.
