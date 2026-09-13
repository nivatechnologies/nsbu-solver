# Bounded endpoint timeout investigation

The source diff from the completed early-clock implementation
`a1d04a7ffc866fa3c826c01eae7269c575ba927a` to the frozen endpoint implementation
`9b4a980f147978b03cece70a265cc97090984c91` leaves the analytical evaluator, 1024-cubed
measurement lattice, retained N512 grid, FFT, measurement loop, worker count, and resource ledger
unchanged. The only runtime-relevant numerical choice is the closed clock: elapsed/remaining ticks
change from 512/7680 to 4096/4096.

The clock reaches several expensive operations. Producer sampling passes it to `scalar::evaluate`,
which calls the safeguarded `root::solve` with the remaining time. Measurement reference sampling
passes it to `reference::evaluate`, whose implicit jet root also begins with `root::solve`, and
regional classification independently calls `root::solve`. Coverage evaluation also uses the
clock, but has only the fixed small panel plans recorded by the diagnostic.

The endpoint process accumulated 73631.04 user seconds before timeout versus 38381.63 user seconds
for the completed early run, about 1.92 times as much. Endpoint wall time was 6049 seconds versus
4168 seconds, already more than 1.45 times as long despite being incomplete. Maximum RSS was nearly
identical. The endpoint process reported zero swaps and one major page fault, so its process
counters do not show a paging explanation. System load and concurrency were not identical between
the runs.

The harness records no producer/reference/derivative phase timings and emitted no partial output.
Therefore this evidence cannot determine which clock-dependent operation dominated, attribute the
runtime increase to a specific cause, or estimate a trustworthy completion time. The endpoint
numerical diagnostic remains unresolved.

The immutable pre-run checksum set is preserved verbatim as `prepared/PREFLIGHT_SHA256SUMS` and
binds the complete evidence tree at commit `21bde21570d49a9092a530c869bdf22e42c3717a`. Verify it by
extracting that commit with `git archive`, changing to this evidence directory inside the archive,
and running `sha256sum -c prepared/PREFLIGHT_SHA256SUMS`. The frozen
`prepared/implementation.json` retains its original pre-execution status; actual execution status
belongs only to `run-clock4096.json` and the current README summary.
