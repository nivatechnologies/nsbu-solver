# N192 h32 versus piecewise time diagnostic

Root approved one bounded invocation. It completed with exit status zero at the
exact admission cap of 344,326,144 bytes. The adapter source is `e59dc06`; the
exact input, binary, command, and output hashes are in `run-provenance.json`.

At physical clock 4096, the h32 state and h64-to-2048/h128-to-4096 state differ
by L2 `6.299803654589401e-9`, H1 `9.956002185670631e-7`, vorticity L2
`9.955802869386301e-7`, and divergence L2 `1.2330697022284534e-14`. The L2,
H1, and vorticity L2 differences are respectively about
`3.425854612815797e-9`, `1.9570538186540516e-8`, and
`1.9582944376441853e-8` of the reported right-state absolute norms. Both sides
have the same N192 retained grid, so full and common-band values coincide and
the newly-resolved shell is exactly zero by construction.

This is a time/execution diagnostic under the recorded
`reviewed-equivalence-supported-by-controls` lineage. Acceptance is
`not_assessed` with zero accepted windows. The result does not establish direct
whole-trajectory exact-source equivalence, arithmetic qualification, or
finest-grid qualification.
