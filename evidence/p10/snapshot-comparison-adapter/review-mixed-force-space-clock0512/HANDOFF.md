# Mixed force/space clock-512 review handoff

This no-run handoff binds the closed three-state diagnostic implemented at `62b763546e70ff59edd2cc56396e72998e31e290`:

- A = U384M384 - lift(U256M384), the spatial increment.
- B = U384M512 - U384M384, the force-resolution increment.
- C = U384M512 - lift(U256M384) = A + B, the combined increment.

The adapter reports volume-average L2, H1, and vorticity L2 norms for A, B, and C on the full N384
band, the N256 common band, and the newly resolved N384 shell. It also reports weighted signed
`2 Re <A,B>` terms and cosine similarities wherever both norms are nonzero.

All three manifests bind existing immutable clock-512 states locally, including exact file and
coefficient hashes, plans, identities, profiles, guards, sources, backends, executions, clocks,
schedules, methods, force grids, tolerances, domains, and case. The exact reservation is
3,138,912,256 bytes: one N256 state, two N384 states, and the fixed 1 MiB adapter allowance.

This explicit mixed diagnostic has no assigned budget and makes no spatial acceptance, force
acceptance, pointwise, time-supremum, finest-grid, or continuum claim. It performs no FFT,
trajectory evolution, state injection, or resume. Root source and semantic-manifest review is
required before the proposed command is run.

The reviewed numerical source temporarily retains `mixed.rs` at 608 lines. Root accepted this quality exception for the single result; any later split is mechanical and must preserve commit `62b763546e70ff59edd2cc56396e72998e31e290` as numerical provenance.
