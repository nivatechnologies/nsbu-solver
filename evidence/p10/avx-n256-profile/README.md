# N256/M384 accelerated profile admission and startup probe

This profile is distinct from the running N192 endpoint. Its endpoint admission retains the same
h32/max128 clock and observer nodes, with integration force M384, explicit observer force M768,
and conservative products on 2N=512. The one-step probe intentionally evaluates the current
observer at startup clock 32 to attribute integration, observer force, conservative, and transfer
cost. That extra startup observation is timing evidence only and is not a scheduled endpoint node.

The exact preflight must pass before the probe launches. The profile is experimental, nonresumable,
and provides no arithmetic, spatial, temporal, quadrature, or PDE-window qualification.

The frozen one-step source is the existing reviewed harness at
`evidence/p10/avx-parallel-reduced-composite-7467e26/harness/src/bin/cached_observed.rs`.
It runs with arguments `256 384 32 32 1 103079215104`; the launch record binds the source commit,
binary hash, profile hash, host headroom, and exact output. The endpoint admission adds the fixed
1,572,864-byte snapshot/history allowance to that one-step owner's reservation.
