# Opt-in intra-transform FFT prototype evidence

This evidence measures the isolated `ParallelFftExecutor` prototype. It does not
change the default FFT backend, qualify a trajectory, or establish end-to-end W3
speed. The production deployment gate remains exact W3 output, zero steady
allocations across more than 63 operations, and an actual one-attempt timing.

## Source bindings

- Base: `b38e02e4daf7da1c11255f4b166bda363531dc85`
- Initial executor: `b09fb7719c66cfddb04e56a37fe3f0d0fadba5a5`
- Axis-2 slab batching and constructor readiness: `649c25ca217371cd306f99d656f1b53de525c4a5`
- Initial benchmark binary: `da2b756fd4454c3cc99dba52fc8b13f1b16f5f6a78e366d10c72bd0dbba11386`
- Batched benchmark binary: `3cd7ce1eeb64d4fc0b3fc0c68dfda993dec0e5dbe67337df25bbd7e74b55212d`

The archived binaries were built from `harness/src/main.used.rs`. The current
`harness/src/main.rs` adds deterministic elapsed phase receipts before every
long operation for any future bounded run. The run harness uses deterministic
dense real input, performs untimed serial and
parallel warmups, verifies canonical little-endian output-word hashes, then
times one serial and parallel forward and inverse transform. Allocation regions
cover each timed parallel call after warmup.

## Results

At the initial source, N96 and N256 workers 8 and 16 produced identical serial
and parallel words and zero allocations/deallocations/reallocations in both
measured parallel calls. N256 workers 8 reduced forward 251.742 ms to 159.905 ms
and inverse 273.319 ms to 177.533 ms. Workers 16 reduced forward 266.139 ms to
160.333 ms and inverse 274.577 ms to 156.681 ms.

The first N768 workers-8 run used row-grain axis-2 tasks and timed out after the
fixed 300 seconds with exit 124, no JSON output, and 24,812,544 KiB maximum RSS.
Its raw `time` receipt is retained in `raw-n768/`; the shell exited before it
could create a separate status file.

After batching axis 2 by whole x slabs, N768 workers 8 completed with exit 0:

| direction | serial | parallel | reduction |
|---|---:|---:|---:|
| forward | 12.369856 s | 6.368704 s | 48.5% |
| inverse | 13.399075 s | 6.364011 s | 52.5% |

Forward and inverse hashes matched exactly and both measured parallel calls had
allocation tuple `[0,0,0]`. Whole-process wall time was 297.12 seconds and
maximum RSS 24,812,544 KiB; allocation/page-touch and untimed setup dominate
that wall time, so only the emitted transform phase timings support the speed
comparison. No N768 workers-16 run was attempted.

## Allocation limitation

These scalar measured calls do not prove persistent allocation behavior. The
independent three-concurrent-caller test performs repeated foreign
`ThreadPool::install` submissions and observes one 1,520-byte allocation and
deallocation after warmup. That size matches a crossbeam injector block. The
standalone executor therefore has no zero-steady-allocation claim. The reviewed
W3 design instead enters each persistent lane loop into the pool once at startup
and makes numerical transforms from inside the pool; that separate integration
must pass its own greater-than-63-operation allocation guard before deployment.
