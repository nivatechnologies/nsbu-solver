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

## Completed runs

All four bounded invocations completed with full-spectrum finite, Hermitian and
Nyquist validation. The original artifact was then regenerated with the same
source-bound binary under `/usr/bin/time -v`; every repeated artifact has the
same complete-file SHA-256 as its original. RSS is the repeat's process maximum,
not the conservative admission estimate above.

| M | Original force (s) | Repeat force (s) | Repeat wall | Max RSS (KiB) | Coefficient SHA-256 | Artifact SHA-256 |
|---:|---:|---:|---:|---:|---|---|
| 384 | 9.290477717 | 9.324537040 | 0:16.38 | 3,943,424 | `8ae626e40923f632e5e4a5912707e6e77ea94ad7731a2af50310e6fa4da72a99` | `125e6fad532e8f8a5bb5f9103c5cebd9c38577672b5a31ea5d5f3c7121feafef` |
| 512 | 24.273537816 | 24.158231007 | 0:34.23 | 8,796,160 | `e05c722e6108a96bf72187a8ebba10566b54226d1175208e043a647a1ac5db7c` | `1faf5a29f85802a3ed8737ea27354b042ff66db2683c53d603f5d90a278603b6` |
| 576 | 30.240206401 | 30.241831701 | 0:42.53 | 12,354,560 | `76b21be045c9a6a03f2f21b2c731ec9c8201a8a09c3c9560b84907da00ffed53` | `913977c2bc2e929b6dbeca132d93725c17958183ccf98a5c6bcbfba06920508d` |
| 768 | 81.507092782 | 81.088092306 | 1:43.65 | 28,730,368 | `d43672a7f2707a1baae8eb929f0f7b99cdd427b70a9375beb36cba25d38bda89` | `ffcb6e29d37dc3451332321d6212df069b92911ca3d481116eb2d1be009deb21` |

Each raw artifact is 405,799,160 bytes. Original and repeat files remain in the
untracked local archive `evidence/p10/force-grid-ranking-de6dd09/work/`. A
deterministic `zstd -19 --long=31 -T1` test on M768 produced a 314,758,182-byte
file with SHA-256
`c08c61a25dfda2b8b062fd5369529508cd9860c3d436741fc19ded258a075ae1`.
`zstd --long=31 -t` passed and decoded SHA-256 equals the raw artifact hash.
The compressed artifact is deliberately excluded from Git because it is still
larger than 100 MB.

## Successive-grid ranking

Raw differences use the full N256 retained band. Since the retained layout is
identical, `common == full` and `newly_resolved == 0` for every channel. The
ratios below divide each raw difference by the corresponding full norm of the
right (finer-sampled) force spectrum. The force itself is not divergence-free;
divergence L2 here is a comparison channel rather than a zero-validity target.

| Pair | Raw L2 | Raw H1 | Vorticity L2 | Divergence L2 | Fine L2 denominator | Fine H1 denominator | H1/fine H1 |
|---|---:|---:|---:|---:|---:|---:|---:|
| 384→512 | 2.6573533083354905e-1 | 2.548492163435638e2 | 1.2204134261023593e2 | 2.2372743049915204e2 | 3.441409087691699e3 | 5.562169809399458e5 | 4.58183092348055500e-4 |
| 512→576 | 7.429956886732749e-3 | 7.35419169721482 | 3.6573347402514322 | 6.38028078692291 | 3.441409087733932e3 | 5.562169812879642e5 | 1.32218036209279536e-5 |
| 576→768 | 1.4353731956535851e-3 | 1.4219347758497258 | 7.284901765681117e-1 | 1.221146391456288 | 3.44140908773239e3 | 5.562169812668622e5 | 2.55643898647443286e-6 |

The finest M768 full norms are L2 `3.44140908773239e3`, H1
`5.562169812668622e5`, vorticity L2 `5.561985650019187e5`, and divergence L2
`2.9399459193585985e3`. Full ratio vectors in `[L2,H1,vorticity,divergence]`
order are:

- 384→512: `[7.72170131659032663e-5, 4.58183092348055500e-4, 2.19420455874736543e-4, 7.60989947441511744e-2]`
- 512→576: `[2.15898682699915780e-6, 1.32218036209279536e-5, 6.57559183042755555e-6, 2.17020326612397604e-3]`
- 576→768: `[4.17088802598408852e-7, 2.55643898647443286e-6, 1.30976637195315053e-6, 4.15363555980887821e-4]`

For ranking the dynamically active part, the analyzer also projects each raw
difference with the full-band Leray projector, retaining the zero mode. Nyquist
storage slots are skipped consistently with the solver's modal operations.

| Pair | P(delta f) L2 | P(delta f) H1 | P(delta f) divergence L2 | Stokes-response L2 | Stokes-response H1 |
|---|---:|---:|---:|---:|---:|
| 384→512 | 1.373642655365512e-1 | 1.22041419908147e2 | 2.2908999838938765e-14 | 2.0226427643052632e-7 | 1.6099503324584343e-4 |
| 512→576 | 3.949974434528694e-3 | 3.6573368730636404 | 6.651861591091293e-16 | 5.784911901887924e-9 | 4.515701605124716e-6 |
| 576→768 | 7.759035493206544e-4 | 7.28490589721808e-1 | 1.2694457732454635e-16 | 1.0608487243400595e-9 | 8.6937503472728e-7 |

The Stokes-response channel applies
`(1-exp(-|k|^2*T))/|k|^2` to `P(delta f)` at `T=4096/2^20=1/256`, with zero
mode factor `T`. It is a ranking-only linear response diagnostic. It supplies no
nonlinear error bound and does not qualify a force grid or PDE trajectory.

The complete machine-readable values, commands, identities, timings and local
archive paths are in `summary.json`. The analyzer source is
`c8aae6a79d6f15b54d71631382a9cd6c27fec2d3`; its predecessor `c507e0c` failed
the first comparison with `InvalidIndex` because it attempted to project
Nyquist-only storage slots. That refusal is retained in the summary, and no
result from the failed analyzer is used.
