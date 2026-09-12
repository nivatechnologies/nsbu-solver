# FFT arithmetic backends

`FftBackend::OwnedRadix` remains the default for every existing constructor. The opt-in
`RustFft6_4_1AvxFma` profile fixes the dependency version, the x86 feature set (AVX, AVX2 and
FMA), normalization, axis order, and the admitted one-dimensional lengths:

`6, 96, 128, 144, 192, 256, 288, 384, 512, 576, 768, 1024, 1152, 1536`.

No generic RustFFT planner or runtime-selected fallback enters this profile. Construction refuses
a missing CPU feature or any length outside that table. The execution catalog plans both
directions for the complete table before constructing numerical workspaces. All FFT owners clone
immutable plan `Arc`s from that catalog and retain separate mutable grids and scratch.

The catalog reservation charges 1 MiB plus 64 allocator/header bytes for each of the 28
length/direction plans, plus every catalog entry. At the largest admitted length, 1 MiB is more
than 42 complete 1536-element complex vectors per direction. This bound is deliberately much
larger than the spike's measured retained planner allocation; the measurement is not used as a
production preflight guarantee. The pinned finite-catalog source audit and allocator measurement
in `evidence/p10/avx-fft-storage-audit-7467e26.md` found 204,544 retained requested bytes and
207,860 total requested construction bytes. The 29,362,480-byte reserve is therefore a tested
fixed accounting allowance for this version, target, allocator, recipe order and length table.
It is not an allocator-enforced or portable proof; any change to those inputs requires a rerun.

Each scalar workspace separately charges its retained half-grid, one input row, one output row,
four maximum-length scratch rows, the fixed plan/workspace objects, a boxed six-plan `Arc` header,
and 64 bytes of allocator overhead. Construction queries all six plans' actual
`get_inplace_scratch_len()` values and refuses any recipe requiring more than the admitted four
rows. Caller input/output arrays, force buffers, operator arrays, persistent force-worker stacks,
queues, headers, and their existing allocator allowances remain in their owning reservations.
Transform calls use `process_with_scratch` and allocate no steady-state storage.

`v2_run::Plan` stores and exposes the selected backend. Version-one v2 archives and reconstructed
runs accept only `OwnedRadix`; attempting to archive or reconstruct an accelerated plan fails
closed. An accelerated run therefore cannot silently resume through an artifact that promises the
original arithmetic profile. Public CLI selection and an accelerated archive format remain
outside this opt-in seam.
