# RustFFT AVX three-dimensional feasibility spike

This bounded spike compares the source-`f6346da` owned mixed-radix FFT with an
isolated RustFFT 6.4.1 wrapper. It does not change a workspace dependency, public
API, runtime selection, state, checkpoint, or accepted numerical result.

The wrapper uses the explicit `FftPlannerAvx` and refuses a machine without AVX
and FMA. This host also reports AVX2. It preserves the current three-dimensional
phase order (contiguous z rows, transverse x rows, transverse y rows), retained
half-grid layout, forward division by the complete real-grid size, inverse
Fourier-sum normalization, and ascending scalar row traversal. RustFFT changes
the one-dimensional algorithm and floating operation order, so output hashes
are expected to differ.

An independent N=6 direct DFT gives maximum scaled forward errors
`1.687878415925942e-16` for the owned radix and `1.3184513337203707e-16`
for RustFFT. The interleaved warm-plus-three-pair profiles used four scalar
transforms per timing. Median wall speedups were 7.963 at N=192, 8.161 at N=288,
and 7.492 at N=384. The maximum full-spectrum owned/RustFFT discrepancy across
the recorded pairs was `2.0563e-16`, `2.9185e-16`, and `1.2877e-16`, respectively.
Round-trip physical differences were at most `4.4011e-15` on the profile fields.
Both backends together made zero allocator calls during the steady transform
probe. Peak joint RSS was 2,664,764 KiB.

RustFFT reports exact `process_with_scratch` lengths; each recorded size needed
N complex scratch values. The wrapper accounts for its grid, row, scratch and I/O
buffers. Planner construction made 11 allocations and 5 deallocations per size;
the measured retained-byte proxy rose from 6,528 to 12,608 bytes. This is not a
production pre-allocation reservation: RustFFT does not expose a bound for
planner-owned twiddles, algorithm objects and caches. Production admission first
needs a finite supported-length policy and a source-audited conservative storage
table for the exact dependency and feature set.

For the proposed higher-reference ladder, the gain is material but does not make
the runs short. A transform-plus-force-sampling projection gives an optimistic
38.0% step reduction for N192/M384/h16 and 51.5% for N256/M384/h16. Their
256-step endpoint lower bounds remain about 58.7 and 63.8 hours with RustFFT,
before nonlinear and diagnostic reductions. N384/M768/h8 is not currently
admissible: its observer requests M1536, while the current FFT contract caps an
axis at 1024, and the projected storage/work is much larger. Its extrapolated
512-step lower bound is roughly 955 hours even with RustFFT.

The next implementation review should prefer this backend spike over W=3 scalar
batching. A production candidate should be an explicit arithmetic profile with
RustFFT 6.4.1, AVX/FMA/AVX2 requirements, supported lengths, planner recipe and
storage-table identities recorded in plan/checkpoint/provenance data. A shared
top-level plan catalog should reuse immutable forward/inverse plans across the
many RHS, force and observer FFT owners. The existing radix constructor remains
the default until the new arithmetic profile passes full operator, force,
trajectory, allocation, refusal and archive controls.

