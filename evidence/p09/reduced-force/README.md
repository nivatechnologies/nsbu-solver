# Optional reduced-coordinate force evaluator

The [public API and equations](../../../docs/REDUCED_FORCE.md) implement the
unchanged exact-v2 prescribed force with degree-three Taylor polynomials in
w=x*x+y*y, z and time. The default runtime providers remain unchanged. This
increment supplies pointwise values, with no force-gradient or PDE qualification.

The [measured report](summary.json) records nine earlier independent fixtures and
84 exact-coordinate/dyadic-clock cases, each recomputed in the independent Python
Cartesian evaluator at 80 and 120 digits. Maximum scaled precision change is
4.42016e-79; maximum Rust discrepancy is 3.59940e-14 against a 5e-12 regression
limit. All 19 outputs are compared: velocity, raw pressure, force and the twelve
separate momentum terms. The symbolic calculation independently verifies twelve
Cartesian transport identities plus zero divergence of the poloidal potential.
These algebra and pointwise checks do not bound untested force spectra or states.

Thirty-two focused tests/probes execute, including 27 library tests (four new),
three fixture/contract tests, the example and an isolated allocator executable.
All ten new Rust files measure 98.35% executable line and 92.86% branch coverage,
with maximum CRAP16. Whole-workspace static maxima remain CC21, cognitive18,
Halstead75.8956 and 477 physical lines. Strict Clippy, formatting and Rustdoc pass. Repository/link/frozen-input checks,
41 bootstrap tests and the original mathematical verification also pass.
Full source-matched hosted coverage/CRAP gates remain required before publication;
the earlier alpha/family reports retain their original verification scopes.

A fresh source export reproduces all 448 maintained source hashes, all 84 fixture
studies and the complete fixture bytes. Focused tests, the allocator, the N16
public example and packaging all three crates pass there. Every numerical and
preflight line of the example reproduces exactly. Its measured median pointwise
speed ratio is about 6.014, with maximum Cartesian/reduced difference 2.00478e-13.
This sequential shared-host profile excludes caches, FFTs, workers and integration;
it is not a measured solver speedup. Numerical calls allocate nothing after entry.

[Source hashes](source-sha256.json) and [artifact hashes](artifact-sha256.json)
bind the complete reports and raw execution logs. Compressed artifacts retain
the original fixture generator, symbolic calculation, original execution reports,
focused LLVM/RCA/CRAP reports, and clean-source replay scripts. The clean replay
changes only the original generator's absolute checkout path; both script hashes
and identical result rows are retained in `clean-reproduction.json.gz`.

Run the committed Rust checks from the checkout root:

```sh
cargo test -p nsbu-benchmarks --lib reduced_force
cargo test -p nsbu-benchmarks --test reduced_force --test reduced_force_allocation
cargo run --release -p nsbu-benchmarks --example reduced_force_profile -- --dry-run
cargo run --release -p nsbu-benchmarks --example reduced_force_profile
```

The historical Python scripts require Python 3.12 with `requirements-dev.txt`.
Decompress them into `work/` before inspection. Their preserved absolute paths
identify the measured environment; a replay on another host must change source
and output paths, retaining the original scripts and reporting the changed script
hash. The fixture calculation admits 168 evaluations under a 256 MiB address-space
cap; the symbolic calculation uses a 512 MiB cap. The archived clean replay
demonstrates identical numerical results after the checkout-path substitution.

P08/P09/P10 remain incomplete. An admitted provider, full Fourier comparisons,
trajectory comparisons and arithmetic-choice evidence are still required before
runtime use. Zero concentrating PDE windows are accepted.
