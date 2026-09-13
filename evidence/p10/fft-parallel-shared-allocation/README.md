# Shared FFT pool allocation diagnostic

The dedicated `fft_parallel_allocation` executable runs three persistent external
callers against one 8-worker pool. Each caller prepares its inputs and scalar
reference before measurement, warms four forward/inverse roundtrips, then executes
four measured roundtrips. Thread creation and teardown are outside measurement.
Every measured forward and inverse result matches the scalar reference bitwise.

At numerical source `b09fb7719c66cfddb04e56a37fe3f0d0fadba5a5` the executable
failed with one allocation and one deallocation of 1,520 bytes. The retained
stdout, stderr and status are from the second reproduction. The same unchanged
fixture also failed at `649c25c` after constructor-wide worker startup and slab
batching; this later invocation was observed directly, not retained in these logs.

Source inspection identifies a matching mechanism: rayon-core 1.13.0 submits
foreign-thread `ThreadPool::install` calls to its crossbeam-deque 0.8.8 Injector.
That queue allocates successive blocks of 63 JobRef slots, each 24 bytes, plus
an 8-byte link: 1,520 bytes per block. The measured call sequence crosses a block
boundary. This is a source-based attribution, not an allocation stack trace.
Constructor warm-up cannot eliminate recurring block turnover.

This is a failed candidate check, not an exemption from the steady allocation
contract. The numerical trajectory running on Sulaco uses the previous backend
and is unaffected.
