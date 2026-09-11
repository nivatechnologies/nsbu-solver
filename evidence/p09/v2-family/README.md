# Independent exact-v2 refinement family

This increment adds six independently evolved exact-v2 trajectories with one
fixed force sample grid. Spatial, temporal and CM/HO comparisons include every
finer Fourier mode and preserve the means. All branches start at exact rest.
See the [API and runnable example](../../../docs/V2_REFINEMENTS.md).

The [measured report](summary.json) records six harness tests, one isolated
allocation probe and the example test. Focused coverage across all eight new
Rust files is 98.71% of executable lines and 85.42% of branches; maximum CRAP is
19.6133. Fresh whole-workspace static maxima are CC21, cognitive18,
Halstead75.8956 and 477 physical lines. Format, strict Clippy, Rustdoc and the
type-identifier scan pass. Repository/link/frozen-input checks, all 41 bootstrap
guard tests and the original mathematical verification also pass. The 98 unchanged Python sources retain the alpha's
220-test profile. Full current-source hosted coverage/CRAP gates are pending;
these focused results do not replace that release requirement.

The independent oracle explicitly sums signed full-complex Fourier modes using
its own indexing, conjugation, wave numbers and accumulations. It checks all five
actual endpoint pairs: full/common/new L2, H1, curl and divergence norms plus
preserved means. Identity fixtures separately check the canonical little-endian
encoding. Invalid plans and terminal failures retain their resource and committed
state contracts. Measured construction heap is 19,219,520 bytes within the
24,190,424-byte reservation; admission and steady sampling allocate nothing.

The release example uses N=[4,8,12], fixed M=12 and steps [64,32,16], with exact
quantum 2^-20 and endpoint 128 (t=1/8192). At that endpoint the two full H1
spatial differences are 7.79974e-6 and 6.92665e-6. The temporal differences are
3.46355e-7 and 2.25943e-8, and CM/HO differs by 2.51210e-18. These values belong
to a short startup diagnostic. They do not show adequate spatial/force resolution
or reach the first concentrating endpoint t=1/256. A preliminary longer/coarser
profile stopped on its local H1 indicator; the final test uses smaller steps.

[Source hashes](source-sha256.json) bind every maintained source to this result.
[Artifact hashes](artifact-sha256.json) cover compressed raw LLVM/RCA/CRAP reports,
test/lint/documentation logs, the complete example output and reproduction scripts.
Decompress the focused-check script to inspect its exact commands; its scratch
paths assume the recorded worktree layout. LLVM coverage includes exercised
dependencies, while the focused totals select exactly the eight new files listed
in the report. P08/P09/P10 remain incomplete, with zero accepted PDE windows.

Both [hosted Rust](hosted-rust.json) and [hosted Python](hosted-python.json)
workflows pass on implementation commit b42c0ca, including complete workspace
tests and all required quality gates. Raw hosted logs are retained. Their
coverage thresholds pass; the numerical percentages above remain explicitly
scoped to the focused local report. Later CI changes preserve raw hosted
coverage and complexity files as downloadable artifacts.
