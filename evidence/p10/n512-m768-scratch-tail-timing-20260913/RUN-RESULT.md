# N512/M768 scratch-tail timing result

The one authorized attempt ran from 17:38:15Z through 18:14:34Z and exited
zero. It completed the exact rest-to-clock-64 Cox--Matthews interval in
1982.648986471 seconds inside the integrator and 1884.472128906 seconds inside
12 RHS calls. The cache recorded five misses and seven hits, the provider
consumed 4,690,292,918 work units and 135 scalar transforms, and the steady
allocation count was zero.

The local error words exactly equal the earlier equivalent-input run. The L2
word is `78cacf927e0c513e`; the H1 word is `c320a0dfcc37673e`. A local accepted
token was present. Its read-only candidate coefficient hash is
`2fdef176af2ca28e6d78183075539e8204095120a0d2975694bec240679d5e13`.
The old result did not record this hash, so no old/new candidate hash equality
is claimed. The token was dropped: no state was committed, written, or
published.

`/usr/bin/time` measured 2179.18 seconds wall time and 155,286,528 KiB maximum
RSS, with no swap or major page faults. The earlier single run measured 2209.71
seconds wall, 2067.912943179 seconds integration, and 1969.460301335 seconds
RHS time. The new integration and RHS measurements are 85.263956708 and
84.988172429 seconds lower, respectively. These are single samples under
different ambient host conditions and do not establish a general speedup.

All owned processes were absent after exit. This remains isolated timing and
resource evidence, not production adoption, PDE qualification, or an accepted
window.
