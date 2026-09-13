# Ordered-Hessian screen results

All four preauthorized FFT-free screens completed in the required sequence with exit status 0 and empty stderr. The unit-volume domain makes L2 and RMS numerically equal.

| Clock | Pair | Difference RMS | Fine absolute RMS | Relative | Cap bytes |
|---:|:---:|---:|---:|---:|---:|
| 512 | N192→N256 | 3.780153779077679 | 215.3965655930855 | 0.01754974026010666 | 578,486,272 |
| 512 | N256→N384 | 1.200280094488947 | 215.3999097826941 | 0.005572333320379977 | 1,772,879,872 |
| 4096 | N192→N256 | 59.68537472703738 | 3406.649505459523 | 0.01752025696549796 | 578,486,272 |
| 4096 | N256→N384 | 19.00922602299126 | 3406.702540739857 | 0.005579948878912951 | 1,772,879,872 |

The adjacent relative ratio contracts by `0.31751656935` at clock 512 and `0.318485561593` at clock 4096.

Acceptance remains `not_assessed`; no Hessian budget is assigned. These screens make no pointwise, time-supremum, finest-grid, or continuum claim.
