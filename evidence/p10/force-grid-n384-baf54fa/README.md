# N384 M384 versus M512 exact-v2 force sampling rank

This isolated, source-bound diagnostic compares physical force sampling M384
and M512 at fixed retained N384 and exact clock 4096. It uses the opt-in
`RustFft6_4_1AvxFma` backend and `ParallelReducedV2Force` with 32 workers.

The outputs rank M384 versus M512 for a possible future N384 pilot. They do not
establish force sufficiency, a force error budget, reduced-arithmetic
qualification, nonlinear error, or PDE trajectory qualification.

The two samples run sequentially under the harness's 96 GiB cap and an outer
timeout. Because long endpoint jobs were active, sampling uses `nice -n 10` and
all timings are labeled contention-affected. Raw spectra remain in the ignored
local `work/` directory; only hashes and small evidence are tracked.
