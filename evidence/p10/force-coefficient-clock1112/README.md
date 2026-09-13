# Clock-1112 retained-force sampling discriminant

This focused force-only harness extends the historical N384 force-grid analyzer without changing
that evidence. It samples the maintained parallel reduced exact-v2 provider at the exact residual
probe clock 1112 on M384, M512, M768, and conditionally M1024, always cropping to strict N384.
It evolves no trajectory and changes no equation, reference state, frozen plan, or acceptance
status.

For each adjacent artifact pair, `compare` reports raw and Leray-projected differences on disjoint
max-mode shells: inside N48, N48--N64, N64--N96, N96--N128, N128--N192, and N192--N384. It also
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

Actual sample runs require the separately reviewed host resource plan. Artifacts are create-new,
source/case/clock bound, and refused on mismatch. They remain local because each retained N384
artifact is about 1.37 GB. All results are empirical binary64 diagnostics without interval
enclosure, force sufficiency, PDE qualification, accepted interpolation, or accepted window.
