# Clock-1112 retained-force sampling discriminant

This focused force-only harness extends the historical N384 force-grid analyzer without changing
that evidence. It samples the maintained parallel reduced exact-v2 provider at the exact residual
probe clock 1112 on M384, M512, M768, and conditionally M1024, always cropping to strict N384.
It evolves no trajectory and changes no equation, reference state, frozen plan, or acceptance
status.

For each adjacent artifact pair, `compare` reports raw and Leray-projected differences on disjoint
max-mode shells: inside N48, N48--N64, N64--N96, N96--N128, N128--N192,
N192--N256, and N256--N384. It also
reports the same projected difference after the explicitly labeled linear Stokes response over
`1112 * 2^-20`. Contracting M384--M512 and M512--M768 differences supports improved convergence
of the sampled-force coefficients on the retained band. It does not prove that the M768-only shell
is a true continuum tail; sampling aliasing can remain. M1024 is a conditional rung if the first
two differences are inconclusive.

The existing offline residual localization measured `P(f768-pad(f384))` H1 as
`935.561405170177` on strict N384 and `1932.914846937184` on the N384--N768 shell. The retained
part is about 18.98% of the total H1-squared difference, so an omitted-shell-only explanation is
already excluded. Its residuals nearly reproduce those force differences: base M768 H1 is
`935.5579161334231` retained and `1932.9048270592857` on the shell, whereas the discrete M384
control is `0.7830415812306416` and `0.441153443692872`. The new adjacent sampling pairs test
whether the retained discrepancy contracts; they do not turn residual acceleration H1 into a
velocity H1 error or acceptance budget.

Build and preflight from this directory with the exact source binding:

```text
RUN_SOURCE=$(git rev-parse HEAD) cargo test
RUN_SOURCE=$(git rev-parse HEAD) cargo build --release
RUN_SOURCE=$(git rev-parse HEAD) cargo run --release -- sample 384 work/m384.bin --dry-run
RUN_SOURCE=$(git rev-parse HEAD) cargo run --release -- sample 512 work/m512.bin --dry-run
RUN_SOURCE=$(git rev-parse HEAD) cargo run --release -- sample 768 work/m768.bin --dry-run
RUN_SOURCE=$(git rev-parse HEAD) cargo run --release -- sample 1024 work/m1024.bin --dry-run
```

Actual sample runs require the separately reviewed host resource plan. The sampler refuses a final
path that exists before evaluation, and the actual unique paths did not collide. Its final
`rename`, however, is not a no-clobber publication primitive: a concurrently created destination
could be replaced between the precheck and rename. Artifacts are source/case/clock bound and
refused on mismatch. They remain local because each retained N384
artifact is about 1.37 GB. All results are empirical binary64 diagnostics without interval
enclosure, force sufficiency, PDE qualification, accepted interpolation, or accepted window.

## Result

The reviewed sequential M384/M512/M768 ladder completed at source `f7bb588` with zero swaps.
The full strict-N384 Leray-projected H1 difference fell from `935.8629343304178` for
M384--M512 to `22.289249597245494` for M512--M768, an adjacent ratio of
`0.023816788527043` (about a 42-fold contraction). The initial N192--N384 outer shell gave
essentially the same contraction, from `935.7371539965881` to `22.28445957063686`. The projected
Stokes-response H1 proxy contracted from `6.134187955851721e-4` to
`1.351263724368082e-5`.

This fixed-retained contraction directly supports coarse M384 sampled-force aliasing or
discretization sensitivity in the scalar parallel-reduced provider. The archived residual used a
width-three W3 cached provider. Existing controls compare that W3 path bit-for-bit with the scalar
AVX path on the admitted N4/padded6/force6 constructor fixture, and the separate admitted M512
force control compares every serial/W3 coefficient word and complete `ForceWork`. There is no
direct same-source, same-clock M384 scalar/W3 comparison bound to this ladder. Applying this result
to the archived W3 residual therefore relies on those execution-equivalence controls and the shared
provider arithmetic. With that caveat, the exact archived replay and nearly one-for-one
residual/force-delta H1 values disfavor reconstruction or operator assembly as the main cause. It does not prove every
archived coefficient error is bounded, that M768 is force-sufficient, or that its new shell is a
continuum tail. M1024 was therefore not run. Full identities, timings, hashes, and limitations are
in `summary.json`; compact captured sample receipts and rerun analyzer stdout are in `run-logs/`.

Eight independently checked exact modal fixtures were also requested through the maintained local
Qwen controller. The first overlarge-context packet validated one task after repair and exhausted
seven tasks after 15 total repairs; that failed batch is preserved. The explicitly authorized
compact redesign validated the remaining seven tasks, five on the first attempt and two after one
local repair each. All eight outputs are consumed by the Rust shell/norm test table. These fixture
results validate finite analyzer arithmetic cases only and make no scientific acceptance claim.

The later Qwen projection-helper experiment produced no candidate. Its first request is preserved
as a client byte-cap failure. The corrected request also failed its output schema after its one
authorized repair. Its task packet additionally says `cos(2*pi*(x+4y))` while expecting retained
mode `[1,2,0]`; the matching expression would be `cos(2*pi*(x+2y))`. That inconsistency prevents a
model-capability conclusion from this failed packet. The receipt and faulty packet remain preserved.
