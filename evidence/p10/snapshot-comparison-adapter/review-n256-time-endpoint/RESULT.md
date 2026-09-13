# N256 h32 versus piecewise time diagnostic

Root approved one bounded invocation. It completed with exit status zero at the
exact admission cap of 812,646,400 bytes. The adapter source is `e59dc06`; the
exact input, binary, command, and output-archive hashes are in
`run-provenance.json`.

At physical clock 4096, the h32 state and h64-to-2048/h128-to-4096 state differ
by L2 `6.299858537050901e-9`, H1 `9.956006384425095e-7`, vorticity L2
`9.955807065221843e-7`, and divergence L2 `1.0904124107440259e-14`. The L2,
H1, and vorticity L2 differences are respectively about
`3.425884452478653e-9`, `1.9570523346789924e-8`, and
`1.9582929490289095e-8` of the reported right-state absolute norms. Both sides
have the same N256 retained grid, so full and common-band values coincide and
the newly-resolved shell is exactly zero by construction.

This is a time/execution diagnostic under the recorded
`reviewed-equivalence-supported-by-controls` lineage. Acceptance is
`not_assessed` with zero accepted windows. The result does not establish direct
whole-trajectory exact-source equivalence, arithmetic qualification, or
finest-grid qualification.
