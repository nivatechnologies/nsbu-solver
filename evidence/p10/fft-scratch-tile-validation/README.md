# AVX scratch-tail tiled FFT validation

Source `0843b8b` stores the eight-lane transverse tile in the tail of the existing AVX
scratch allocation. It preserves `FftWorkspace`'s public layout and leaves OwnedRadix
allocation and traversal unchanged. Validation commit `9eba11f` corrects three stale test
constants only; harness commit `3afdfdd` adds private diagnostics.

One scalar AVX workspace grows by exactly `8 * 768 * 16 = 98,304` bytes at N768. The W3
additional forward and bidirectional reservations each grow by 196,608 bytes because they
contain two additional workspaces. Actual API preflight reports 50,117,086,952 bytes for
the tiled force control and 72,386,085,024 bytes for the tiled RHS control. Both remain
inside the historical 64/96 GiB caps.

The planned scalar diagnostic repeats the frozen untiled A, tiled A, tiled B, untiled B
order with canonical little-endian word hashes. If those hashes remain identical and the
predeclared exploratory speed heuristic passes, only the new tiled force and RHS controls
need execution. Their hashes must equal the already frozen canonical untiled hashes:
`a5a74f81...a65b` for force and `7fbbe90c...5c73` for RHS. Each control also requires
internal serial/W3 word equality and zero steady allocations.

Cheap prelaunch controls match every frozen untiled forward and inverse hash for
anisotropic `[6,96,192]` and `[96,6,192]` sparse inputs and dense `[96,6,6]`, under both
OwnedRadix and AVX. These cover unequal axes plus one-lane and four-lane partial tile
tails. The scalar harness is byte-identical to the earlier hash harness. The W3 harness
diff contains only the two scratch-tail reservation constants and their two delta checks.

The scalar N768 diagnostic completed with identical hashes across all four rounds. Median
forward time fell from 13.8865 s to 9.0182 s and inverse from 14.0578 s to 9.1843 s; the
combined median improved 34.86% with neither direction regressing. This passes the
exploratory selection heuristic. It is validation of an isolated optimization candidate,
not production adoption or a PDE acceptance gate.

The subsequent tiled force and RHS controls also completed. Both internally matched
serial and W3 coefficient words, made zero steady allocations across three repeats, and
matched the frozen untiled canonical hashes exactly. No swap or major fault occurred.
Production adoption remains conditional on the independently running maintained-workspace
regression and final integration review.
