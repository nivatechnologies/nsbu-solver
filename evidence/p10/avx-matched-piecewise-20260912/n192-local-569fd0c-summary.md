# Matched piecewise N192 endpoint archive

Run `n192-local-569fd0c`; source commit `569fd0ced7a755b1f6c066a7c00016f537fbc4bf`; case SHA-256 `e1236f7b3c51537acd17381402ca420ba7872a7b9dbc64b2f0d9d5108a468f7e`. Profile: N192 retained, M384 integration force, M768 observer force, 32 sampling workers, serial component AVX RHS, 3 force workers, rustfft 6.4.1 AVX/AVX2/FMA, Cox–Matthews; Cadv 3.3; profile `n192-m384-h64to2048-h128to4096-cadv33-avx-force-w3-f13c29c`.

Schedule is exact: attempts 1–32 use h64 for [0,2048), attempts 33–48 use h128 for [2048,4096), endpoint clock 4096. All 48 attempts are committed, cover the exact contiguous tick ranges, and have zero steady allocations; no refusal records or refusal output were present. Observer nodes are 0, 512, 1024, 1536, 2048, 2560, 3072, 3584, 4096. Terminal stdout reports `endpoint_complete_qualification_pending clock=4096`; timed exit status is 0.

Identity hashes: plan `d03ffea53ed6f41ab7e38e4962b15ca8f5213b304641ce41816ed1d7840bd4f1`; binary `e08b00e6ee420a20a6541b1f4b386fe19e7db0dbace3fdeaf5c9e0ff1689324b`; watchdog `4b65e13b74bd7a32237044b5467d4f223c8dc3e6e35d6e8d3498e1938fa53e4b`; W3 source `f13c29c9ae91d0b8cf7a790132deb9bd076911c0`; preflight stdout `4d024a73177330b60f8001001d4870c99eba986dfd9110c2844b5667f6d0349f`.

Exact runtime hashes: stdout `57bc990ff105413195506d4ed53bc84ce0cf79dd19dab61fbd96ce74e200f2ad`; time `1f671465e6678c31066d5365bfb9452697e5b0556f17fb6bd224efc0e8552e16` (elapsed 1:32:15; max RSS 46,946,304 KiB); stderr `e3b0c44298fc1c149afbf4c8996fb92427ae41e4649b934ca495991b7852b855`; watchdog log `32c3e33a71832c42f1a8e124addf42e3b71f5b09270de9300d220e80590e2442`; launch record `7cc41ac890d4b660c7b1482f67f2b3a2268e2ffd62491fdcf8861dc67a193970`; admission `513a2a82389a218d0bc348fca4a2a270f76166458115ba1bdb4ce823ab50f4b0`.

Raw state bundles remain local and are excluded from Git. The inventory binds every attempt JSON, record JSON, and state payload hash. This is packaging evidence only; qualification remains experimental/pending and no numerical interpretation is asserted.
