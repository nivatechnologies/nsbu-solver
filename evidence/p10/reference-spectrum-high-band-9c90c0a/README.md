# Higher-band binary64 analytical-reference assessment

This read-only follow-up evaluates the maintained explicit scalar v2 analytical velocity at clock 4096 on M192 and M384 periodic grids, retaining strict N192 spectra. It extends the separately preserved M96/M192/N96 assessment. No PDE state is constructed or evolved, and no state is injected or reset.

The M384 H1 shell magnitudes are 5.74909897 on N48→N64, 3.58513938 on N64→N96, 1.07314099 on N96→N128, and 0.42403608 on N128→N192. The strict N192 H1 norm is 50.87239851. High-mode content decays above N48 but remains measurable through the final observed shell. The last shell is about 0.83% of that total; this ratio uses the sampled analytical numerical state as denominator and is not an error bound.

The M192→M384 discrepancy over strict N192 is L2 0.0001004466 and H1 0.06600747. This is materially below the earlier M96→M192/N96 discrepancy, but it neither bounds modes beyond M384/N192 nor qualifies binary64 arithmetic. The maintained scalar implementation is independent of the jet evaluator, not an independent high-precision reference.

All final generation and comparison commands completed with exit zero in 87.03 measured wall seconds total, inside the ten-minute numerical cap. M384 used 2,441,384,184 admitted owner bytes under a 3 GiB owner cap and 4 GiB per-process RLIMIT_AS; measured maximum RSS was 2,383,528 KiB. The separate comparison reserved 686,559,232 bytes and used 25,030,656 coefficient visits. RSS is observational and not an allocation reservation.

The signed-mode normalization, scalar-versus-jet, finite and conjugacy controls pass at both grids. Raw FFT Nyquist content and k=0 defects are printed before strict transfer. M384 maximum k=0 defect is below 2.27e-16; no mismatch is silently discarded. Strict spectra pass the unchanged validator after the explicit Nyquist-excluding transfer.

This diagnostic does not establish trajectory error, spatial convergence, force sufficiency, a continuum spectral tail, or PDE qualification. Accepted windows remain zero. Complete raw output, time records, exact source and binary hashes, commands, and result hashes are retained. Large intermediate coefficient files are hash-bound and reproducible in about 80 seconds from the retained source; they are not committed.
