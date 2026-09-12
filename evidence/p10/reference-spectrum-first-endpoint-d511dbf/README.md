# Binary64 analytical-reference spectral assessment

This read-only assessment evaluates the maintained explicit scalar implementation of the fixed v2 analytical velocity at clock 4096 on M96 and M192 periodic sample lattices. It applies the maintained normalized FFT and compares the strict N96 spectra. It constructs and evolves no PDE state and performs no state injection or reset.

The scalar implementation is independent of the jet evaluator used by reference tracking, and selected axis/interior/exterior values agree with that evaluator under the recorded binary64 allowance. It is not an independent high-precision reference. These sampled transforms do not establish a true spectral-tail bound, binary64 reference precision, trajectory error, convergence, force sufficiency, or PDE qualification. Modes beyond M192/N96 are not bounded. Accepted windows remain zero.

The frozen case SHA-256 is `e1236f7b3c51537acd17381402ca420ba7872a7b9dbc64b2f0d9d5108a468f7e`; source is commit `d511dbfb8f5ae7681cdeaef6131ae7d198315bc7`, tree `1cf4d18d4e73da163455013ce8fb45c624741449`. Exact clock words are exponent -20, target 8192, elapsed 4096, remaining 4096.

M192 gives H1 shell magnitudes 8.81998945 on N24→N32, 13.12833260 on N32→N48, and 6.77535387 on N48→N96, against 50.85931131 inside strict N96. These high-mode contributions remain material and support investigating grids above N48. The M96→M192 difference on strict N96 is L2 0.00266284 and H1 0.88600609, so M96 sampling itself is not settled over the full retained N96 band.

The final M96 and M192 runs passed finite, signed-mode normalization, conjugacy and scalar-versus-jet controls. Raw FFT Nyquist content is measured and printed before the intentional strict-band transfer; it is never silently treated as a valid strict spectrum. Maximum k=0 Hermitian defect was below 1.85e-16. The strict transferred spectra then passed the unchanged 1e-12 validation.

The M96/M192 owner reservations are 57,435,384 and 306,055,416 bytes under a 384 MiB owner cap. They include all simultaneously live three-component real grids, FFT plan/workspace, raw complex output, retained three-component N96 spectrum, report state and allocator allowance. RLIMIT_AS was 512 MiB per process. Measured walls were 1.13 s and 8.38 s; maximum RSS was 57,488 and 300,188 KiB. These RSS values are observations, not allocation reservations.

Two early harness attempts failed closed with `InvalidSpectrum` because raw FFT Nyquist entries were submitted directly to the strict-spectrum validator. Their logs are retained. The frozen harness instead audits and reports raw Nyquist and conjugacy defects, then uses the maintained strict-band transfer. No tolerance was changed and no mismatch was projected away without being reported.

Complete stdout, time records, failure controls, commands, maintained-source hashes, harness source, and result hashes are retained. The 21.7 MB intermediate spectra are bound by hashes and reproducible in seconds from the retained harness; they are not committed as large artifacts. Clippy and Rustdoc pass with warnings denied.
