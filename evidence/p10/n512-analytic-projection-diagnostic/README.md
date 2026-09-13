# N512/M1024 analytical projection regional diagnostic

This is a separate, state-free representation diagnostic. It samples analytical velocity on the
unshifted periodic 1024-cubed lattice, applies the same normalized forward transform and strict
band transfer used by the validated N384 projection, retains N512 coefficients, drops all producer
scratch, and calls the same velocity, ordered-gradient, and ordered-Hessian regional measurement
loop. The existing N384 actual-state and projection commands, bindings, dimensions, and resource
constants remain unchanged.

The result origin is `sampled_analytic_projection`; its explicit sample and retained dimensions
identify the N512/M1024 diagnostic. The interface accepts no snapshot,
manifest, coefficient file, import, or resume argument. Its record contains zero state inputs,
trajectory-from-rest claims, and import/resume interfaces; acceptance, continuum accuracy, collar
volume coverage, and peak qualification remain unassessed.

The allocation-free preflight is:

```text
REGIONAL_MANIFEST=evidence/p10/n384-regional-snapshot-diagnostic/harness/Cargo.toml
cargo run --release --manifest-path "$REGIONAL_MANIFEST" -- projection-n512-preflight
```

The producer has two nonoverlapping subphases. Sampling worker stacks are joined before projection
allocates the FFT plan, workspace, spectrum, and retained coefficients. The larger projection
subphase is exactly 46246603464 bytes:

```text
physical samples             25769803776
FFT catalog                     29362480
FFT workspace                 8606810520
full sampled spectrum         8606711808
retained N512 coefficients    3233808384
producer allowance                  40960
serialization                       65536
total                         46246603464
```

After producer scratch drops, the exact measurement reservation is 305085516888 bytes:

```text
retained N512 coefficients      3233808384
FFT catalog                       29362480
derivative workspace           25803457328
packed analytical cache       257698037760
two magnitude arrays           17179869184
region labels                   1073741824
32 two-MiB worker stacks          67108864
allocator allowance                  65528
serialization                         65536
total                          305085516888
```

The reviewed execution plan requires a fresh `MemAvailable >= 322265386072`, 32 workers, exact
soft and hard `RLIMIT_AS=343597383680`, an exact internal cap of 305085516888, and a 6000-second
timeout followed by 60 seconds of TERM/KILL grace. The measured N384/M768 runtime was 1884.70
seconds; cubic lattice scaling predicts roughly 2.37 times as many samples, so the bounded planning
estimate is 75--80 minutes. No heavy execution is admitted until root review and explicit local
resource handoff.

The frozen implementation source is `a1d04a7ffc866fa3c826c01eae7269c575ba927a`.
Its release binary SHA-256 is
`992505c58def855cc4f81483411c610af9909a2f32a938f80bb6da047186ae2f`; the archived
allocation-free preflight SHA-256 is
`b96e88b1dcf02a38270cd8adc5cf4bcecb3f6eb26d98ed5d075441f392221159`.
All 16 focused tests and clippy with warnings denied pass. Dry runs refuse both a mismatched cap and
a missing root-review gate before allocating the producer, and neither creates an output file.
