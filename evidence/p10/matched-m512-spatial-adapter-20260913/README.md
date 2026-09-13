# Closed N384/N512 M512 spatial adapter

This source-only extension adds `MATCHED_M512_SPATIAL_DIAGNOSTIC` to the
existing snapshot comparison adapter. It binds the completed r6 N384/M512
source/profile to the prepared N512/M512 capture source/profile and fixes the
shared clock, schedule, method, force grid, case, tolerances, domain, and
admission guards. Existing comparison modes and numerical arithmetic are
unchanged.

The adapter reserves exactly 4,600,889,344 bytes: 1,366,032,384 bytes for the
N384 state, 3,233,808,384 bytes for the N512 state, and the existing 1 MiB
bounded overhead. It reports full, common-band, newly-resolved-shell, and fine
absolute L2/H1/vorticity/divergence norms with `acceptance=not_assessed`.

The focused closed-contract test and the complete 23-test adapter suite pass.
Clippy passes for all targets with warnings denied. No trajectory, snapshot,
large allocation, or numerical comparison was executed.
