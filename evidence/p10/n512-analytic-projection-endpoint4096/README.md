# N512 analytical projection at endpoint clock 4096

This source-only variant extends the existing state-free N512/M1024 analytical projection with a
closed clock choice: early clock 512 or endpoint clock 4096. Both clocks use exact exponent -20 and
target 8192; their checked remaining ticks are 7680 and 4096. The analytical evaluator, unshifted
1024-cubed sampling grid, normalized FFT, strict N512 transfer, regional measurement, source hashes,
and resource ledger are unchanged.

The existing clock-512 commands remain explicit. New endpoint-only commands are
`projection-n512-endpoint-preflight` and `projection-n512-endpoint-execute`. Execution still requires
the exact cap and `--root-reviewed`; this preparation performs no heavy run. It accepts no state,
snapshot, trajectory, resume, or reference-injection input and makes no acceptance claim.

The endpoint projection is intended to test whether the N512 representation remains adequate at the
first legal endpoint before committing to a long N512 trajectory. Even a favorable analytical
projection is a spatial diagnostic, not a trajectory or continuum qualification.

The frozen source is `9b4a980f147978b03cece70a265cc97090984c91`, based directly on the
unchanged early-clock source `a1d04a7ffc866fa3c826c01eae7269c575ba927a`. The endpoint variant's
release binary SHA-256 is `bdf43acd1926f45ae5b066679e59753653f19473d38bf21585009a715ca432a5`.
The endpoint and early control preflight records are identical after removing the exact clock field.

`SOURCE_SHA256SUMS` is source-specific and does not rewrite the historical N384 or live early-N512
manifests. To verify the frozen source independently, extract `git archive
9b4a980f147978b03cece70a265cc97090984c91` into an empty directory, change to that directory, and
run `sha256sum -c` using this variant's `SOURCE_SHA256SUMS`. `prepared/implementation.json` binds the
binary, both preflights, resource gates, and zero heavy executions.
