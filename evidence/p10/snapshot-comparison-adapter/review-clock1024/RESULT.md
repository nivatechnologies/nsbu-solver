# Clock 1024 matched-prefix comparison

The bounded adapter completed successfully for all three matched states. N192,
N256, and N384 use the same Cox--Matthews, M384, tolerance, and h64 evolution
prefix through clock 1024. Execution backend and observer metadata remain
distinct and are reported in the raw outputs.

| Pair | L2 | H1 | Vorticity L2 | Absolute divergence |
|---|---:|---:|---:|---:|
| N192 to N256 | `5.0978782965431976e-5` | `3.8112701187268797e-2` | `3.8112667094532406e-2` | `2.3765429726032698e-15` |
| N256 to N384 | `9.351465479960296e-6` | `9.199401893048562e-3` | `9.199397141891092e-3` | `2.002901614437525e-15` |
| N192 to N384 | `5.1829412722417726e-5` | `3.920723878398577e-2` | `3.9207204533639545e-2` | `2.1572533993141e-15` |

The N256-to-N384 H1 difference is `3.933381013572074e-4` relative
to the N384 H1 norm. It is `0.9833452533930185` of the allocated pilot
spatial H1 budget, `4e-4 * ||u_N384||_H1`, at this clock. The vorticity
counterpart is `0.9839977227776828` of its analogous allocation. The velocity
L2 difference is `1.0977187281454161e-5` relative and `0.274429682036354`
of the velocity L2 allocation, `4e-5 * ||u_N384||_L2`.

The H1 pair reduction from N192-to-N256 to N256-to-N384 is
`0.2413736525219456`. The newly resolved shell contains
`0.999999949383104` of the squared fine-pair H1 difference; its common-band H1
difference is `2.0696997476497504e-6`. The coarse N192-to-N384 H1 difference
is `1.6763808171490894e-3` relative and `4.190952042872723` times its spatial
allocation. These shell splits show that common-band-only comparisons would
miss nearly all of the measured spatial differences.

The runs took 9.79 seconds and 30.00 seconds for N192-to-N256 and
N256-to-N384, with peak RSS 565,248 KiB and 1,731,584 KiB. The earlier direct
N192-to-N384 control took 26.62 seconds with peak RSS 1,503,232 KiB. All three
exited successfully without swap.

This is an actual trajectory spatial diagnostic at one interior clock. It is
not an accepted-window result: the maximum over the window, endpoint pair,
other observables, and the remaining acceptance channels are unresolved.

The staged snapshots are deliberately excluded from Git. Their file and
coefficient hashes, plan hashes, source identities, and transfer review are
recorded in the reviewed manifests and raw outputs.
