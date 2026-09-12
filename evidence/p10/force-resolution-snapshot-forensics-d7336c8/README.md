# Exact-v2 force-resolution snapshot forensics

This bounded, read-only study compares already-evolved velocity coefficient snapshots. It performs no evolution, state injection, reference reset, pressure comparison, PDE qualification, or acceptance review. The N12/M96 run completed clock 4096; the independent N16/N24 M96 controls published clock 2048 and then hit their fixed external timeouts before endpoint publication. No accepted window is claimed.

## Bound inputs

All runs start from rest for the same `similarity-mms-v2` case (`e1236f7b3c51537acd17381402ca420ba7872a7b9dbc64b2f0d9d5108a468f7e`), unit periodic domain, viscosity 1, clock exponent -20 and target 8192. The selected force-resolution comparisons use Cox–Matthews with a 16-tick step and the original exact-v2 force. The complete M48 family also contains the other time-step and method branches covered by the 12-pair ordinary/probe identity audit. The coefficient files contain components 0, 1, 2 in axis-major order and each complex coefficient as little-endian `f64` real then imaginary bits; metadata is deliberately excluded from the coefficient hash.

- M24: source `487c3307037be09e19d151324a2bf96dbc92e83b`, N12, 12 workers, uncached provider, absolute tolerances `[1e-5,1e-4]`, relative `[0,0]`. The accepted branch-0 archives bind 128/256 accepted fixed steps at clocks 2048/4096.
- M48: source `5fa65efbad0cc340efdf126278feaf579e0017fa`, N12/N16/N24, 32 workers, attempt-local cached provider, the same absolute and zero relative tolerances. The completed five-event run reached clock 4096 in 7652.919993 s.
- M96: source `def4730b08025fdd06e7a8a0d78116aea24b6e2c`, N12/N16/N24, 32 workers, attempt-local cached provider, absolute tolerances `[1e-5,1e-4]`, relative `[1e-5,1e-5]`. N12 completed clock 4096 with 256 attempts and 256 commits in 1:24:34. N16 and N24 each record 128 attempts and 128 commits through clock 2048, showing no rescheduling at the compared clock. Under the fixed 5100 s timeout, N16 stopped after 254 commits at clock 4064 and N24 after 250 commits at clock 4000. Neither emitted an endpoint snapshot; both runs are partial. The M96 profile metadata is not equal to the M24/M48 profiles.

The previously captured uncached N12/M48 clock-2048 control (12 workers) and the cached M48 branch-0 snapshot (32 workers) have the identical coefficient-only SHA-256 `42b06216f27cc12121b3fb8e1203b98e4a877f5a0aa3fd9e8269a48bb089f015`. This establishes state-word identity for that force/grid/clock across those two executions only. There is no corresponding uncached M48 endpoint snapshot.

A separate read-only audit compares cached M48 ordinary-family and probe-family integrated states. All six branch coefficient files are byte-for-byte equal at clocks 2048 and 4096 (12 pairs). This supports that the reconstruction observers did not alter these cached integrated states; it is not an uncached/cached equivalence claim at 4096.

## Results

The full norms use the production `ComparisonPlan` band convention. L2 is the complete retained velocity difference; H1 includes the velocity and its first spatial derivatives. Vorticity and divergence are also retained in `summary.json` and raw output. Same-N comparisons have no newly resolved band.

| Clock | Pair | Full L2 | Full H1 |
|---:|---|---:|---:|
| 2048 | N12 M24 → M48 | 0.3332248325130536 | 9.868231645532276 |
| 2048 | N12 M48 → M96 | 0.0321524942034136 | 1.0364331867313983 |
| 2048 | N12 M24 → M96 | 0.3568502757200065 | 10.507604723100457 |
| 4096 | N12 M24 → M48 | 0.6146240932112198 | 15.37145537156965 |
| 4096 | N12 M48 → M96 | 0.05474123303374244 | 1.5562026002973386 |
| 4096 | N12 M24 → M96 | 0.6589897222087554 | 16.45286993798289 |

The adjacent M48→M96 difference is 0.09648888998137338/0.10502724540324618 of the M24→M48 L2/H1 difference at clock 2048 and 0.08906457400284541/0.1012397696040949 at clock 4096. This is empirical decay between these snapshots. The M96 relative tolerances differ, and these two clocks do not establish convergence, force sufficiency, or a continuum error bound.

Fixed-M48 spatial comparisons show that both full L2 and H1 decrease from the N12→N16 pair to N16→N24 at both retained clocks:

| Clock | Spatial pair | Full L2 | Full H1 | Common-band H1 | Newly-resolved H1 |
|---:|---|---:|---:|---:|---:|
| 2048 | N12 → N16 | 0.42972937753559176 | 20.908857435687306 | 0.20235501129990904 | 20.907878221266216 |
| 2048 | N16 → N24 | 0.3059723175049371 | 20.426346293484904 | 0.2341248705638876 | 20.425004490730178 |
| 4096 | N12 → N16 | 0.44870572350476157 | 21.510148642289924 | 0.3971913742343494 | 21.50648120045771 |
| 4096 | N16 → N24 | 0.3184693174456582 | 21.08261830913625 | 0.34447485009697854 | 21.079803885387022 |

The M96 midpoint spatial results show the same directional decrease:

| Clock | Spatial pair | Full L2 | Full H1 | Common-band H1 | Newly-resolved H1 |
|---:|---|---:|---:|---:|---:|
| 2048 | N12 → N16 | 0.4310546617012411 | 21.023313706952496 | 0.2002279898232641 | 21.022360190355126 |
| 2048 | N16 → N24 | 0.3082363098359834 | 20.568202615924786 | 0.23426737536152537 | 20.566868445307282 |

This differs from the fixed-M24 endpoint diagnostic, whose H1 increased from 22.213794282100803 to 26.79566622262152 when moving between the same spatial pairs. The force-resolution change is associated with a materially different observed spatial H1 trend in these cross-profile diagnostics. The M96 spatial endpoint remains unavailable here, and these two-level differences do not prove convergence.

Earlier archived M48 states do not supply an easier resolved interval. At clocks 64, 128, 2048 and 4096, N12→N16 H1 differences divided by the finer numerical state's H1 norm are 0.5492, 0.5484, 0.5283 and 0.5005. The corresponding N16→N24 ratios are 0.4720, 0.4711, 0.4587 and 0.4406. L2 ratios likewise stay near 0.25 and 0.18. The tiny absolute errors at clocks 64 and 128 track the tiny field amplitude; they do not represent materially better relative spatial resolution and cannot redefine the design's first concentrating endpoint. `relative-resolution.json` gives the exact values and clearly labels the denominator as the finer numerical state, not an analytical reference.

## Next diagnostic candidate

The chosen higher-grid ladder that extends the existing data is N24/N32/N48 at fixed M96, 32 workers, CM step 16 and endpoint 4096. Independent `Plan::from_rest_cached` preflights admit N32 and N48. Including one coefficient output buffer and 4 KiB I/O allowance, N24/N32/N48 require 686,794,472 / 735,810,664 / 935,669,096 bytes per process; their sum is 2,358,274,232 bytes, but no hard aggregate cap is inferred. M96 equals 2N for N48, and the working plan admits the padded integration/force transfer. Full-band postprocessing for N24→N32 and N32→N48 reserves 2,394,112 and 7,204,864 bytes with 92,096 and 282,624 coefficient visits.

This ladder would only test whether relative full-band decay begins above N24. N48 still falls below the design's twelve-cell first-endpoint heuristic, which raises a resolution concern rather than imposing an acceptance gate. Even favorable results remain unqualified because this study covers only spatial and force diagnostics and does not establish the full mandatory observable inventory. A cheap analytical-reference spectral-tail assessment should precede or accompany it if a source-independent reference transform is admitted; current denominators are finer evolved numerical states, not analytical spectra.

## Comparator and controls

The comparator is compiled against source `d7336c877880c28247c4658b6b9631554ebc4762`. It validates exact input lengths and supplied SHA-256 values, runs the unchanged strict `validate_spectrum(..., 1e-12)` guard, and independently implements the production complete/common/newly-resolved band traversal. Different retained grids include fine-only modes. Its conservative owned reservation is 197,632 bytes for N12/N12, 322,048 for N12/N16, and 944,128 for N16/N24, below its 256 MiB cap; respective coefficient-visit bounds are 7,056, 12,240, and 36,864.

For both M48 spatial pairs at clocks 2048 and 4096, every printed full L2/H1/vorticity/divergence value exactly matches the corresponding production `ComparisonPlan` event text. A self-comparison returns exact zeros. Wrong hash, truncated input, nonfinite spectrum, and wrong archive clock controls all terminate nonzero.

Focused source checks pass: rustfmt, Clippy with `-D warnings`, and Rustdoc with `-D warnings`. Rust-code-analysis over every nested node reports maximum function cyclomatic 20, cognitive 8, all-node Halstead difficulty 58.20952380952381, and file SLOC 376. This archived evidence harness has no global CRAP or coverage claim. The repository policy check passed after the evidence was added.

Raw command output, `/usr/bin/time -v` records, controls, metrics, complete input snapshots, and source are retained here. `artifact-sha256.txt` excludes itself and verifies the frozen evidence bytes.
