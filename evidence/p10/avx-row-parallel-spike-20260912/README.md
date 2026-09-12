# AVX row-parallel FFT evidence spike

This standalone prototype tests the design in `docs/design/parallel-row-avx-fft-proposal.md` without changing solver code or public interfaces. It preserves the scalar 3D arithmetic: axis order, one RustFFT 6.4.1 AVX call per row, row element order, retained-half layout, forward normalization, and inverse Hermitian reconstruction.

Three persistent component-owner threads each coordinate themselves and three persistent helpers. The 12 transform workers run inside the harness process. Only a component coordinator reads or writes its grid. It packs up to 256 rows into owned buffers, releases a four-party FFT barrier, waits for all participants, then scatters the completed rows. Two slots per participant alternate between waves. This prototype deliberately does not overlap packing with the prior wave; the benchmark therefore measures the safe bounded owner protocol actually implemented.

## Controls

The N6 fixture compares every spectral and physical `f64` word against the serial RustFFT implementation, checks a direct DFT with maximum scaled error `1.36743736571894098e-16`, verifies zero steady allocations, and injects a caught helper panic. The failure wave drains, the triplet owner terminates permanently, no later request dispatches work, and the external publication generation remains unchanged. Private packed rows, grid, and scratch are unspecified after failure. The caught-panic guarantee is limited to transform/helper failures; this harness has no caller callbacks.

At N384 and N576, serial W3 and row-parallel W3 produced identical SHA-256 hashes for every component after forward and inverse execution. Both modes reported zero steady allocations. Each size used one warm cycle and one measured forward/inverse composite.

| Length | Separate-owner dispatches per 3D triplet | Serial composite | Row composite | Speedup | Measured row non-FFT fraction | Decision |
|---:|---:|---:|---:|---:|---:|---|
| 384 | 3,468 | 1.307003214 s | 1.628431908 s | 0.802615x | 72.894% | fail |
| 576 | 7,794 | 3.972600625 s | 5.228691953 s | 0.759769x | 71.085% | fail |

The non-FFT fraction is `(pack + atomic dispatch + completion-barrier excess beyond the slowest participant FFT + scatter + publication) / (that overhead + FFT barrier phase)`, summed over the three component owners. It is an owner-work fraction; concurrent component sums are not divided by triplet wall time. Full phase totals and hashes are in `raw/profile-384.log` and `raw/profile-576.log`. The stale gate-line overhead fraction in `raw/profile-384.log` and `raw/profile-576.log` divided aggregate component overhead by triplet wall time; the table applies the corrected formula to the unchanged raw phase fields. No benchmark was repeated for this reporting correction.

The run was bounded with a 24 GiB virtual-memory limit, a 1,500-second timeout, `nice -n 19`, and CPU set 52-63, which are 12 physical cores on the single-node host. Two preserved endpoint experiments were active and unbound during these measurements, using about 106 GiB and substantial CPU. This contention qualifies absolute timing and may penalize the 12-worker mode more than serial W3. The observed margins are nevertheless far below the required 1.8x speedup and 35% overhead gates, so this architecture does not qualify for production work from this evidence. A future reassessment would need a clean host and a design that avoids serial full-grid pack/scatter, such as safe direct disjoint row access or a persistent transposed layout.

## Resource audit

The corrected dispatch formula is `C * (ceil(L^2/B) + 2*ceil(LH/B))`, with `C=3`, `B=256`, and `H=L/2+1`.

The earlier 133,901,280 B combined candidate omitted separately allocated row-owner shared state. The concrete prototype adds one shared `Arc<Shared>` allocation per component. Its payload, two-word `Arc` control-header estimate, and 64-byte allocation allowance add 1,728 B per triplet owner. The audited declared increments are therefore:

| Owner | Slots | Helper scratch | Helper stacks/system/metadata | Coordinator metadata | Vector allocation allowance | Shared Arc payload/control allowance | Exact declared increment |
|---|---:|---:|---:|---:|---:|---:|---:|
| L384 triplet | 37,748,736 | 221,184 | 19,482,624 | 3,312 | 2,304 | 1,728 | 57,459,888 B |
| L576 triplet | 56,623,104 | 331,776 | 19,482,624 | 3,312 | 2,304 | 1,728 | 76,444,848 B |
| Combined | | | | | | | 133,904,736 B |

The allocation instrument measured row-specific retained heap of 37,974,377 B at L384 and 56,959,337 B at L576. Thread stacks are reserved outside the Rust global allocator. Immutable forward/inverse plan `Arc` clones allocate no new control blocks: the owner shares two plan control blocks. The declared vector and metadata allowances cover the 4,457 B measured heap remainder above slot and helper-scratch payload at both lengths.

## Reproduction

```sh
cargo build --release --manifest-path evidence/p10/avx-row-parallel-spike-20260912/harness/Cargo.toml
cargo run --release --manifest-path evidence/p10/avx-row-parallel-spike-20260912/harness/Cargo.toml -- validate
(ulimit -v 25165824; exec timeout 1500s nice -n 19 taskset -c 52-63 evidence/p10/avx-row-parallel-spike-20260912/harness/target/release/p10-avx-row-parallel-spike profile-384)
(ulimit -v 25165824; exec timeout 1500s nice -n 19 taskset -c 52-63 evidence/p10/avx-row-parallel-spike-20260912/harness/target/release/p10-avx-row-parallel-spike profile-576)
```
