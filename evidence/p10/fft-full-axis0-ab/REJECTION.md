# Full-axis0 FFT candidate rejection

The opt-in full-axis0 transpose candidate is rejected. On the same Baccus host and dense deterministic N768 input, the existing axis1/2 parallel profile completed three exact forward/inverse rounds with median transform times 9.909 s and 8.974 s. The full-axis0 candidate emitted three exact forward times (median 18.652 s) and two exact inverse times (median 16.486 s) before its frozen 300 s timeout. That is 1.882x and 1.837x the axis1/2 times, respectively.

Every emitted transform reported zero allocations. All emitted forward and inverse hashes match the existing large N768 oracle (`be113a…` and `a5b035…`). Peak RSS was 49,604,608 KiB for axis1/2 and 60,249,088 KiB for full-axis0. The extra transpose storage and strided copies therefore regress this measured case despite preserving numerical words.

The source bindings are library `4b5a6709`, implementation `5c9c61ed`, harness `d9972fe9`, binary SHA-256 `dad76537…`, and launcher `b7aee2d` / SHA-256 `6bc01c9b…`. Verification used `/usr/bin/openssl` SHA-256 `30cc7c49…`; its exact small oracle and 64 MiB throughput receipt are archived. The original word-at-time and batched-software-hash timeout receipts remain additive evidence of measurement overhead, not FFT performance.

After the first full-axis measurements established the regression, the identity-verified outer coordinator was terminated so unstarted repetitions could not launch. The active full-axis process remained untouched and reached its original timeout. This is a performance diagnostic only. It makes no trajectory, PDE acceptance, or qualification claim, and the rejected production prototype must not be integrated.
