# AVX `from_parts` CRAP remediation candidate

Candidate `0a775f5be1282e068b1c78fbb175387b5ae1af2f` adds one `cfg(test)` unit test and changes no production logic. It supplies an actual RustFFT length-1536 plan to a length-6 layout and verifies that `from_parts` refuses its scratch requirement before workspace allocation. This exercises the meaningful closed-scratch validation outcome that the existing suite missed.

The focused test passes. The source-bound serial package coverage run passes 252 tests with no failures or ignored tests. `from_parts` branch coverage rises from 0.5 to 1.0 and its CRAP falls from 26.125 to 11.0. The maximum CRAP across the five production FFT files is then 19.125 (`avx.rs::from_catalog`), below 25. Static maxima remain within their thresholds: CC 14, cognitive 15, Halstead difficulty 74.667, and file SLOC 283. No exclusions or metric rules changed.
