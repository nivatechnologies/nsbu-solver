# RustFFT AVX deterministic W=3 batching spike

This isolated spike starts from `codex/p10-fft-batch-20260912` at
`f134856cd199e38ea87c595b5492cbf18fef2e4d`. It tests whether three independent
scalar transforms can run concurrently on persistent RustFFT 6.4.1 AVX/FMA
workers without changing any scalar transform arithmetic. It changes no
production module, public/default API, scheduler, transpose, experiment plan, or
active endpoint artifact.

The candidate owns one lane and one mutable `FftWorkspace` per worker. Each lane
has committed and staging physical/spectral buffers. A request submits all three
workers, drains every submitted completion, and only then publishes in ascending
lane order by swapping buffers. Injected worker errors and caught worker panics
drained all completions and preserved the prior published generation. The
steady forward-plus-inverse region made zero allocator calls. Every N=288,
N=384, and N=576 published word equaled the interleaved serial result, repeat
hashes matched, and an independent N=6 direct DFT had maximum scaled error
`1.88917678584948649e-16`.

## Measurements

Each direction contains three complete isotropic three-dimensional scalar
transforms. Medians are from three interleaved serial/W3 pairs. The copy-inclusive
column adds a measured copy of all three result fields into caller-owned buffers;
the swap-published path itself performs no bulk copy.

| length | serial F+I (s) | W3 swap F+I (s) | swap speedup | result copies (s) | copy-inclusive speedup | dispatch |
|---:|---:|---:|---:|---:|---:|---:|
| 288 | 1.638882567 | 0.556292992 | 2.946078x | 0.072917649 | 2.604664x | 13.904 us |
| 384 | 4.315002579 | 1.738235828 | 2.482403x | 0.166618043 | 2.265267x | 21.447 us |
| 576 | 13.971830332 | 4.672695522 | 2.990101x | 0.567298994 | 2.666383x | 12.946 us |

All three lengths clear the isolated 1.8x batch gate, including the full-copy
charge. Peak joint RSS for the serial owner, W3 owner, and copy adapter was
53,856,448 KiB. The complete profiling process used 5:33.70 wall seconds and
exited zero.

## Resource declaration

The fixed reservation sums exact element bytes, production FFT workspace
reservations, three 2 MiB stacks, 64 KiB per-thread allowances, object headers,
and 64 bytes per owned allocation. A forward-only production adapter needs two
additional workspaces and two additional spectral staging lanes; inverse uses
two physical staging lanes. A bidirectional owner retains both staging types.

| length | forward addition | inverse addition | bidirectional addition | complete W3 profiling cap |
|---:|---:|---:|---:|---:|
| 288 | 776,267,520 B | 773,613,312 B | 1,158,473,600 B | 4,065,026,712 B |
| 384 | 1,827,942,144 B | 1,823,223,552 B | 2,733,911,936 B | 9,576,959,640 B |
| 576 | 6,143,131,392 B | 6,132,514,560 B | 9,200,779,136 B | 32,205,022,872 B |

Every exact-cap constructor succeeded, and every one-byte-short constructor
refused before lane or worker construction. The separate full-copy adapter is
included in the profiling cap and reported independently in `profile.stdout`.

## Scheduled-observer decision

The frozen endpoint has 128 h32 attempts and eight positive observers, so its
measured observer cost is amortized at 1/16 per attempt. The source-matched
startup pair averages 110.501877340 s integration and 129.821624112 s per
observer, giving a scheduled baseline of 118.615728846 s per attempt.

The projection replaces only transforms that form explicit triples: for each of
12 RHS calls, two inverse batches and one forward batch at N=288; five cached
force forward batches at N=384; and, optimistically, all nine N=384 conservative
observer transforms as one inverse and two forward batches. It leaves each RHS's
tenth scalar transform, M768 observer force, sampling, modal work, transfer, and
measurement unchanged.

The swap-published projection is 92.530101853 s per scheduled attempt, a 21.99%
reduction. Adding the measured caller copies gives 94.256398534 s, a 20.54%
reduction. Both miss the required 25% integration-step gate. The conservative
observer assumption is optimistic because its six product transforms currently
have dependencies and would require additional product storage, so the result
does not hide a plausible pass.

The decision is negative only for this frozen N192 scheduled-cost model: do not
integrate W3 into that endpoint. It is not a global rejection. The measured
2.27--2.67x copy-inclusive batch result remains promising for N256 and N384,
whose component costs differ. `DESIGN_PROPOSAL.md` records a bounded, design-only
N256 model and an unqualified N384 guard study for root review. Neither changes
the numerical/backend contract, production code, the active endpoint, or a
default API.

`QUALITY.md` records the corrective module split and focused quality evidence.
The original `profile.stdout` and `profile.time` bytes and hashes are unchanged.
