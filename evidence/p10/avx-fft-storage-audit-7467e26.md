# RustFFT 6.4.1 AVX finite-plan storage audit

## Scope and source basis

This audit covers only the closed `FftPlannerAvx::<f64>` catalog introduced by
`7467e26`: both directions of lengths `6, 96, 128, 144, 192, 256, 288, 384,
512, 576, 768, 1024, 1152, 1536`, constructed in that order.  The workspace
dependency is exactly `rustfft 6.4.1`, crates.io checksum
`21db5f9893e91f41798c88680037dba611ca6674703c1a18601b01a72c8adb89`.

The recipe basis is the pinned crate's `src/avx/avx_planner.rs` (local source
SHA-256 `f52a23b4af126f461ae624e4917ff5532ce156ed3fc2d371c426187137bb0d42`):

* `FftPlannerAvx` owns an `FftCache` of forward and inverse maps; constructed
  FFTs retain their internal `Arc` data when the planner is dropped.
* The f64-specific `plan_mixed_radix_base`, `replan_with_cache`, and
  `construct_plan` select the butterfly and mixed-radix chain below.
* `FftCache` source SHA-256 is
  `ab1624bfa0ed41c0dfe9a90288b030ad8975498deb68938f4d31287cbbdee260`.

`B<n>` means the f64 AVX butterfly selected by `construct_butterfly` (except
`B6`, which is RustFFT's scalar `Butterfly6`); `C<n>` is the same-direction
cached plan. `×r` is RustFFT's AVX `MixedRadix<r>xn` wrapper. Both directions
use the same recipe and allocation figures.

| length | recipe after this catalog's cache replan | retained bytes/dir | total requested while constructing alone/dir | scratch elements |
|---:|---|---:|---:|---:|
| 6 | `B6` | 40 | 252 | 0 |
| 96 | `B12 × 8` | 1,728 | 1,941 | 96 |
| 128 | `B128` | 1,952 | 2,164 | 128 |
| 144 | `B36 × 4` | 2,464 | 2,684 | 144 |
| 192 | `B24 × 8` | 3,264 | 3,484 | 192 |
| 256 | `B256` | 3,872 | 4,084 | 256 |
| 288 | `B32 × 9` | 4,896 | 5,109 | 288 |
| 384 | `C32 × 12` | 6,304 | 6,517 | 384 |
| 512 | `B512` | 8,032 | 8,244 | 512 |
| 576 | `C36 × 16` | 9,440 | 9,653 | 576 |
| 768 | `C96 × 8` | 12,640 | 12,854 | 768 |
| 1024 | `C128 × 8` | 16,448 | 16,668 | 1,024 |
| 1152 | `C288 × 4` | 18,752 | 18,972 | 1,152 |
| 1536 | `C192 × 8` | 24,928 | 25,148 | 1,536 |

The cache matters: for example, `1152` initially has the `B36 × 8 × 4`
shape, but the earlier 288 plan is a cached intermediate, so RustFFT constructs
the outer plan as `C288 × 4`. There are no Rader or Bluestein branches in this
finite catalog. Every reported plan exposes `get_inplace_scratch_len() <= length`;
the length-6 scalar `Butterfly6` is the sole zero-scratch exception.

## Allocation measurement

The committed standalone harness under `source/` uses `System` as its global
allocator, wrapping every allocation, zeroed allocation, reallocation, and
deallocation. It measures Rust allocation *requested layout bytes* rather than
allocator usable bytes. `rustc-version.txt`, `platform.txt`, and `allocator.txt`
identify the measured target: rustc 1.94.0, `x86_64-unknown-linux-gnu`, Linux
6.8, and glibc 2.39. `hashes.sha256` pins the harness, its lockfile, raw output,
the production `fft.rs`, and the upstream source inputs.

For every length/direction it creates a fresh `FftPlannerAvx::<f64>`, retains
the returned `Arc`, then drops the planner. It then constructs the production
`FftCatalog` in its exact order. `retained` is live requested bytes after
construction; `peak_requested_live` is the largest live requested-byte value
seen after the measurement baseline and before construction returns;
`construction_total` is the sum of all requested allocation sizes in that
interval. Thus `construction_total` is an upper bound on peak requested live
bytes, whereas `peak_requested_live` is the measured requested-live peak.

The independent fresh-plan rows sum to 229,520 retained bytes over 28 plans.
The actual shared catalog retains 204,544 bytes, has a 206,280-byte measured
requested-live peak, requests 207,860 total bytes during construction, and
makes 86 allocations. The 1,580-byte difference between total and requested-
live peak is transient allocation that was released before construction
returned. The reduction from independently built plans reflects cache sharing
in the prescribed order. The catalog is then dropped cleanly.

`FftCatalog::reservation` at `7467e26` is 29,362,480 bytes:

```
28 * (1,048,576 plan allowance + 64) + 14 * size_of::<CatalogEntry>()
```

It exceeds the measured complete-catalog retained footprint by 143.55×, the
measured requested-live peak by 142.34×, and the construction-total upper bound
by 141.26×. This comparison
includes retained opaque plan graphs, twiddle tables, planner-cache descendants,
`Arc` allocations, map/vector metadata, and transient allocations observed by
the allocator. It does not include scalar FFT workspaces, which are separately
reserved by the production code.

Reconstruct the raw output only from an audited checkout: its production
`crates/nsbu-solver/src/spectral/fft.rs` must match `7467e26` and the pinned
source hashes. From that checkout's repository root, run without network access:

```
git diff --quiet 7467e26 -- crates/nsbu-solver/src/spectral/fft.rs || exit 1
cargo run --locked --offline --manifest-path evidence/p10/avx-fft-storage-audit-7467e26/source/Cargo.toml > /tmp/nsbu-avx-storage-audit.txt
cmp evidence/p10/avx-fft-storage-audit-7467e26/raw-stdout.txt /tmp/nsbu-avx-storage-audit.txt
sha256sum -c evidence/p10/avx-fft-storage-audit-7467e26/hashes.sha256
```

The command was replayed with the locked harness and matched the committed raw
output byte for byte. It is not a claim that a later checkout with changed
production source will reproduce the result. The source tree's ignored
`source/target/` directory is build output and is not evidence.

## Recommendation and limit

The 1 MiB-per-length-direction reserve is empirically conservative for the
entire admitted RustFFT 6.4.1 f64 AVX catalog on the tested target, including
construction rather than only steady state. Retain the numeric reserve for this
finite profile, but document it as a **fixed accounting reserve validated by
the catalog-allocation measurement**, not as an allocator-enforced proof or a
portable bound on RustFFT internals. A future RustFFT version, target,
allocator, recipe-order change, or admitted-length change must repeat this
table and complete-catalog measurement before reusing the reserve.

The profile's existing eager construction prevents a later opaque-plan growth
from appearing during a transform. It still cannot make the process allocator
honor a reservation: normal allocation can fail after preflight under external
memory pressure. That limitation is common to the surrounding vector reserves
and should remain stated separately from this finite-plan audit.
