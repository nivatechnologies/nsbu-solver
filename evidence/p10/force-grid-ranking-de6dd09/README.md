# N256 exact-v2 force sampling-grid rank

This bounded diagnostic asks whether the retained N256 force spectrum at exact
clock 4096 continues changing materially as the physical sampling grid increases
through M384, M512, M576 and M768. It uses the opt-in
`RustFft6_4_1AvxFma` catalog and `ParallelReducedV2Force` with 32 workers from
base source `de6dd09f10ce9a0bd6c788c03da4eebd964210c7`.

The decision-changing outputs are complete same-retained-layout differences for
M384→M512, M512→M576 and M576→M768: absolute L2, H1, vorticity L2, divergence
L2, common/newly-resolved decomposition and component mean differences. These
rank candidate force grids for later trajectories. They do not establish a force
error bound, select a force budget, qualify the reduced arithmetic, or qualify a
PDE trajectory.

Each sample invocation is independently preflighted and writes one transactional
binary spectrum. It refuses an existing final or partial path, writes a `.partial`
file, flushes it, and renames only a complete artifact. The raw format contains a
16-byte magic, format version, exact source and case hashes, retained and sampled
layouts, worker count, exact clock words, coefficient count, then all three
components as little-endian binary64 real/imaginary words. Each successful run
prints separate coefficient and complete-artifact SHA-256 hashes.

The fixed retained output is 405,798,912 bytes. Peak admission retains two such
outputs for comparison and separately counts the 29,362,480-byte immutable AVX
catalog, provider reservation, 1 MiB artifact buffer and 64 KiB metadata:

| Samples | Provider reservation | Admitted peak |
|---:|---:|---:|
| M384 | 3,698,181,816 B | 4,540,256,232 B |
| M512 | 8,668,026,552 B | 9,510,100,968 B |
| M576 | 12,310,970,040 B | 13,153,044,456 B |
| M768 | 29,079,844,536 B | 29,921,918,952 B |

The 96 GiB harness cap is a ceiling; no invocation reserves that amount. Runs are
separate so an outer timeout preserves earlier complete spectra. The intended
commands, after replacing `SOURCE` with the committed harness source, are:

```sh
RUN_SOURCE=SOURCE timeout 600 \
  evidence/p10/force-grid-ranking-de6dd09/harness/target/release/p10-force-grid-ranking \
  sample 384 evidence/p10/force-grid-ranking-de6dd09/work/m384.nsbuforce
RUN_SOURCE=SOURCE timeout 900 \
  evidence/p10/force-grid-ranking-de6dd09/harness/target/release/p10-force-grid-ranking \
  sample 512 evidence/p10/force-grid-ranking-de6dd09/work/m512.nsbuforce
RUN_SOURCE=SOURCE timeout 900 \
  evidence/p10/force-grid-ranking-de6dd09/harness/target/release/p10-force-grid-ranking \
  sample 576 evidence/p10/force-grid-ranking-de6dd09/work/m576.nsbuforce
RUN_SOURCE=SOURCE timeout 1800 \
  evidence/p10/force-grid-ranking-de6dd09/harness/target/release/p10-force-grid-ranking \
  sample 768 evidence/p10/force-grid-ranking-de6dd09/work/m768.nsbuforce

RUN_SOURCE=SOURCE \
  evidence/p10/force-grid-ranking-de6dd09/harness/target/release/p10-force-grid-ranking \
  compare evidence/p10/force-grid-ranking-de6dd09/work/m384.nsbuforce \
  evidence/p10/force-grid-ranking-de6dd09/work/m512.nsbuforce
```

M512→M576 and M576→M768 use the same compare form. Because all outputs have the
same retained N256 layout, the comparator's common result equals its full result
and newly resolved is exactly zero. This is a sampling-grid difference, distinct
from a retained-grid refinement. No N384 output is emitted because the current
provider would require a second transform/evaluation owner; avoiding duplicate
physical sampling would require a separate interface change outside this study.
