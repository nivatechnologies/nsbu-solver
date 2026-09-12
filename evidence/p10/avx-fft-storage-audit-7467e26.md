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
finite catalog. All reported plans expose `get_inplace_scratch_len() == length`.

## Allocation measurement

On this x86_64 AVX/AVX2/FMA host, a dedicated `stats_alloc` process used
`System` as its global allocator. For every length/direction it created a fresh
`FftPlannerAvx::<f64>`, retained the returned `Arc`, then dropped the planner.
It also constructed the production catalog in its exact order. `retained` is
requested allocated bytes minus requested deallocated bytes after construction;
`construction_total` is all requested allocations during construction and hence
an upper bound on the requested-byte construction peak (not an allocator usable
size measurement).

The independent fresh-plan rows sum to 229,520 retained bytes over 28 plans.
The actual shared catalog retains 204,544 bytes, requests 207,860 total bytes
during construction, and makes 86 allocations. The reduction reflects cache
sharing in the prescribed order. The catalog is then dropped cleanly.

`FftCatalog::reservation` at `7467e26` is 29,362,480 bytes:

```
28 * (1,048,576 plan allowance + 64) + 14 * size_of::<CatalogEntry>()
```

It exceeds the measured complete-catalog retained footprint by 143.55× and the
measured complete-catalog construction-total bound by 141.26×. This comparison
includes retained opaque plan graphs, twiddle tables, planner-cache descendants,
`Arc` allocations, map/vector metadata, and transient allocations observed by
the allocator. It does not include scalar FFT workspaces, which are separately
reserved by the production code.

The measurement command was:

```
cargo test -p nsbu-solver --test fft_avx_storage_audit -- --nocapture --test-threads=1
```

The temporary reporting test was not retained in the branch; this document
records its source basis and output.

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
