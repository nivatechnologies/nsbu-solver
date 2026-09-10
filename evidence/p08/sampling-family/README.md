# Actual smooth physical sampling family

P08/P09 remain incomplete. This increment measures complete physical diagnostics
on three sample lattices while every independently evolved trajectory remains
unchanged. The [public guide](../../../docs/SAMPLING_REFINEMENTS.md) documents
the fixed smooth profile, ownership, finite work and interpretation.

[summary.json](summary.json), [source-sha256.json](source-sha256.json) and
[artifact-sha256.json](artifact-sha256.json) preserve the complete measured scope,
execution profile and compressed/original report hashes. The source inventory
includes 265 Rust and 82 Python/stub files. Python source is byte-identical to
the preceding complete 198-test derived-field profile; all 41 bootstrap tests
are rerun on this checkout.

## Numerical and resource results

The example independently evolves N=4/8/12 with macro ticks 64/32/16 and CM/HO
to tick 128 at quantum 2^-16. It samples M=24/32/48 at ticks 0/64/128.
Velocity, the complete gradient/Hessian, vorticity, physical pressure and its
gradient each retain all five spatial/temporal/method comparison pairs.
Each of the **270 complete comparisons** retains its original RMS, sampled
absolute and relative peaks, reference peak, component count and fixed floor.

The joint reservation is **53,685,896 bytes**, below the 128 MiB cap. Three
successful observations charge **5,220 scalar transforms**, **202,176 provider
work units**, and **1,162,808,640 conservative weighted visits**. Weighted visits
exclude FFT internals and are not FLOPs. Caller artifacts and allocator overhead
remain separate. An isolated allocator executable verifies complete admission,
construction, successful evolution/measurement, malformed requests and exhaustion.

All six state digests remain unchanged during measurement. The analytic control
`cos(2*pi*x-pi/8)` has RMS `1/sqrt(2)` on M=4/12/16 while the captured peaks are
`cos(pi/8)`, `cos(pi/24)` and 1. Equal RMS therefore cannot hide a missed peak.
The actual smooth findings include near-cancellation changes; no automatic
convergence rate or supported zero floor is inferred from them.

## Verification and integration

The initial complete profile passes 357 Rust tests/allocation probes on the
pressure-family base. That source is preserved with its raw coverage, metrics,
CRAP and clean export. The independently verified derived-field increment is then
integrated, and the complete combined Rust scope is replayed. The combined **362 Rust tests/probes pass**: 356 harness tests and six isolated
allocator executables. Executable line coverage is **22,809/23,082 (98.817%)**;
instrumented branch coverage is **1,572/1,742 (90.241%)**. Maxima CC21, cognitive18,
Halstead75.8956, physical-file469 and CRAP24.33594 pass every required gate.
Source-matched hosted verification of this sampling increment is pending.

The complete workflow uses the pinned optimized test profile with debug assertions
and overflow checks retained. Formatting, strict Clippy, strict Rustdoc, fresh
public packaging, repository/frozen-input checks and the original mathematical
runner are included. A clean public export matches the maintained source and
reproduces all 270 example findings byte-for-byte. Informational duplication and
dead-code reports retain their findings; no mutation sweep is rerun.

Malformed family input spends an attempt before child work starts. A deliberately
desynchronized private child verifies that a later numerical failure terminates
the aggregate, publishes no partial record and cannot obtain a fresh retry.
Every quantity, pair, sample grid, actual clock and original statistic remains
present in successful reports. Strict grid refinement, full pressure-band
admission, floors, cap refusal, finite work and overflow have negative controls.

Immutable planning, existing independent physical/pressure consumers, complete
publication and sampling-change reporting have separate responsibilities.
Source-matched hosted checks are recorded separately from local execution.

```sh
cargo test -p nsbu-benchmarks --lib sampling
cargo test -p nsbu-benchmarks --test sampling_family --test sampling_allocation
cargo test -p nsbu-benchmarks --example smooth_sampling
cargo run --release -p nsbu-benchmarks --example smooth_sampling -- --dry-run
cargo run --release -p nsbu-benchmarks --example smooth_sampling
```

Complete pinned gate commands follow the repository workflows, with raw local
paths prefixed `work/p08-sampling-refinement-`. This consumer observes synchronized
accepted clocks; streaming arbitrary off-stage time manifests remains a separate
owner. Full reference/force/arithmetic/quadrature and regional/location evidence,
benchmark/artifact binding and concentrating integration remain.
**Accepted concentrating PDE windows: zero.**
