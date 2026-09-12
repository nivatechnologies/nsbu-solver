# W3 integrated-source quality correction

The source freeze is `f13c29c9ae91d0b8cf7a790132deb9bd076911c0` on
`codex/p10-w3-avx-integration-20260912`, based on integrated FFT-split source
`29d9f1ce04e11544a9108286a0863cbd31b30372`.

Focused rustfmt and warnings-denied Clippy pass for `nsbu-solver`,
`nsbu-benchmarks`, and the evidence harness. The six W3 pool controls, two
rotational controls, explicit RHS admission control, four public reduced-force
controls, and one harness whole-attempt cache control pass. They cover consumed
seed identity/capacity refusal, exact cap boundaries, AVX bit equality, every
lane's numerical failure and caught worker panic, drained permanent termination,
no external RHS/pressure/cache/candidate publication on failure, and a later
provider request refusing before fresh point-sampling work.

Rust-code-analysis over every changed Rust file reports maximum function
cyclomatic complexity 19, cognitive complexity 8, and all-node Halstead
difficulty 73.55421686746988. The rotational unit and three split impl nodes are
52.39150943396226, 56.651162790697676, 46.39655172413793, and
54.871428571428574 Halstead difficulty. The largest changed file is 433 physical
lines. Focused branch coverage plus `quality/check_crap.py` measures 135 changed
production functions with maximum CRAP 24.640625 (`from_scalar_lane`, CC 19,
75% branch coverage). No coverage exclusions were used.

The final whole-step confirmation is not an additional statistical pair. It
binds numerical and timing behavior to the final imports and module split. Its
state hash is bit-identical to the pre-split W3 and serial controls, and its
integration time remains consistent with the earlier W3 median.
