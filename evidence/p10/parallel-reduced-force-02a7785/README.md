# Plane-parallel reduced exact-v2 force prototype

This diagnostic tests one hypothesis: sampling the already verified reduced
degree-three force with the existing persistent axial-plane pool will remove the
measured force bottleneck while preserving the serial reduced coefficient words.
It changes no default provider, scheduler, runtime setting, checkpoint format or
PDE acceptance rule. The working branch starts from `02a7785`; the measured
production source is `ffee8b7dac733fb17e41ac7bafb1a7f4498be985`.

The pool dispatch, fixed cyclic plane ownership, collection order, error/panic
drainage and joining `Drop` remain shared with `ParallelV2Force`. Each worker now
owns an explicit Cartesian or reduced sample payload. The reduced payload keeps a
private root per owned plane, visits x/y and local planes in the same scalar order,
and copies disjoint values into the original global row order. The existing serial
reduced FFT and retained-band transfer run only after every plane succeeds.
`ParallelReducedIdentity` records retained and sampled layouts, worker count and
the supplied construction cap.

## Measured force result

Both profiles use 32 persistent workers at exact clock 4096, quantum `2^-20`,
target 8192. Providers are constructed and dropped sequentially while all three
retained outputs remain allocated. Timings cover one complete sample/FFT/transfer
request.

| retained / sampled | original parallel | reduced serial | reduced parallel | parallel-reduced speedup over original | admitted peak |
|---|---:|---:|---:|---:|---:|
| N64 / M192 | 9.443814679 s | 22.834749370 s | 3.006074871 s | 3.1416x | 543,407,528 B |
| N192 / M384 | 69.322047436 s | 196.187730639 s | 23.042640458 s | 3.0084x | 4,213,777,832 B |

The full reduced spectrum matches the serial reduced spectrum bit-for-bit at both
sizes. Every component is finite and passes the repository Hermitian/Nyquist
validator. At M192 the original/reduced difference is L2
`7.079946679536221e-13`, H1 `1.221477512353411e-10` and maximum scaled coefficient
`8.516994471706635e-15`. At M384 those values are
`1.069594072203025e-12`, `5.41326634246041e-10` and
`9.314002514124181e-15`. These are separate arithmetic/provider identities; no
original/reduced bit-equality claim is made.

The M384 provider reservations are 3,698,795,944 bytes for original parallel,
2,269,840,888 bytes for reduced serial, and 3,698,181,640 bytes for parallel
reduced. All report the same conservative 7,304,380,416 work-unit ceiling and
three scalar transforms. Actual work was 56,624,672 units for each provider.

## Bounded trajectory control

After the force gate passed, an isolated harness evolved two independently owned
N64/M192 CM trajectories from exact rest through one accepted 128-tick step. The
original branch took 111.259913604 s and the reduced branch took 47.470672393 s,
a measured 2.3438x step speedup. Each attempt made 12 RHS calls and reported the
same `[12, 84942396, 156]` consumption. Both indicators passed with the same
ratios to displayed precision. The resulting difference was L2
`1.0307006762371699e-22`, H1 `1.0437666041569832e-20`, and maximum scaled
coefficient `2.646977769354886e-23`.

This one-step result demonstrates an actual force-call/cost effect. It is not the
clock-4096 endpoint, a refinement result, or a full-run forecast. The separate
historical N192/M384 h16 runtime baseline of 1126.717 s and 45.334 GB RSS is
retained as context; this packet does not multiply component speedups into that
baseline.

## Verification and reproduction

Focused numerical tests cover 1/2/5/12-worker anisotropic partitions, rest,
backward and repeated requests, exact serial-reduced word equality, finite full
spectra, Hermitian/Nyquist validation and unchanged original-parallel behavior.
Worker error and panic controls drain all completions, preserve unpublished
outputs and terminate the provider. Idle and submitted reduced workers join and
release shared storage on drop. The isolated allocator reports 37 constructor
allocations totaling 50,816 heap bytes for the small three-worker profile; the
6,541,600-byte reservation includes configured stacks, and first/repeated/refused
requests allocate, deallocate and reallocate zero times.

```sh
cargo test -p nsbu-benchmarks --test parallel_reduced_force
cargo test -p nsbu-benchmarks --test parallel_reduced_force_allocation
cargo test -p nsbu-benchmarks --test parallel_force
cargo test -p nsbu-benchmarks --test parallel_force_allocation
cargo test -p nsbu-benchmarks --lib provider::parallel
cargo clippy -p nsbu-benchmarks --lib --test parallel_reduced_force \
  --test parallel_reduced_force_allocation -- -D warnings
cargo doc -p nsbu-benchmarks --no-deps

RUN_SOURCE=ffee8b7dac733fb17e41ac7bafb1a7f4498be985 \
  cargo run --release --manifest-path \
  evidence/p10/parallel-reduced-force-02a7785/harness/Cargo.toml \
  --bin p10-parallel-reduced-force -- \
  64 192 32
RUN_SOURCE=ffee8b7dac733fb17e41ac7bafb1a7f4498be985 \
  cargo run --release --manifest-path \
  evidence/p10/parallel-reduced-force-02a7785/harness/Cargo.toml \
  --bin p10-parallel-reduced-force -- \
  192 384 32
RUN_SOURCE=ffee8b7dac733fb17e41ac7bafb1a7f4498be985 \
  cargo run --release --manifest-path \
  evidence/p10/parallel-reduced-force-02a7785/harness/Cargo.toml \
  --bin trajectory
```

The force harness source hash is
`1a82c483aee0da82c7fa7bbc99790c9280cb113b717924c03b8eabe243dd31b5`;
the executed binary hash is
`cb0861f09fb0644b2c2c16603552dc97e48cd9795c9a153be92b725abcddd0a1`.
The trajectory source and binary hashes are recorded in `summary.json`. Raw force
reports and the pointer-normalized trajectory report are under `runs/`.

Provider identity must cross the `Run`, profile and checkpoint boundaries before
this path can enter the released runtime. Until then, the clean seam is the
separate `PrescribedForce` implementation and isolated harness. A possible
quarter-turn axisymmetry sampler would be another arithmetic/profile identity and
requires its own full-grid comparison; it is outside this prototype.
