# Current-grid reference-arithmetic timing pilot

This fixed pilot times independent 80/120-digit arithmetic for the four exact-v2
tracking quantities at representative points. It does not execute the full grid
or change production code.

The Rust test producer emits binary64 input and output words for velocity (3),
gradient (9), ordered Hessian (27), and curl (3). The Python tool independently
uses the established multiprecision jets. It evaluates both exact centered
12-grid rationals and the exact values of the Rust evaluator's centered binary64
input words. Original uncentered Rust argument words, centered words, exact
dyadic elapsed-time words, and the Rust output words are included in a hashed
12-row stream.

The fixed points are axis `(0,0,1)`, cutoff-collar `(5,0,0)`, interior
`(1,1,1)`, and exterior `(6,0,0)`, with coordinates divided by 12. Each is
evaluated at elapsed ticks 0, 64, and 128 with exponent -20 and target 8192.
Both bindings run at 80 and 120 digits. At 120 digits the tool deliberately
executes the full Python jet at all six legitimate zero rows: four startup rows
and the exterior point's two positive clocks. All are exactly zero in Python
and Rust. Rational, binary-word, and Rust input classes match. Startup zero is a
structural case control and supplies no accuracy floor.

The final pilot completed in 8.44375 seconds internally and 8.58 seconds wall,
with 26,624 KiB maximum RSS under the fixed 60-second and 256 MiB limits. It ran
36 direct multiprecision evaluations, including the 12 zero controls; the
maximum observed scalar root work was eight iterations. The largest active-row
scaled 80-to-120 change was `1.5539161e-84`. The largest rational-to-binary-word
input effect was `2.0071563e-20`, and the largest Rust-binary64-to-word-bound
120-digit discrepancy was `2.7701453e-19`. The last two are measurements with no
new acceptance threshold.

Exact current-grid classification gives 1,213 exterior spatial points. Across
three clocks this is 3,639 exterior rows; startup contains 1,728 rows and
overlaps 1,213 exterior rows. Their union is 4,154 legitimate shortcut rows,
leaving 1,030 active spacetime points. A full two-binding, two-precision study
would therefore execute 4,120 multiprecision jets and 5,184 Rust evaluations.
Its admitted scalar-root ceiling is 8,437,760 iterations, plus exactly 12,360
three-step jet corrections.

The 24 representative active evaluations averaged 0.255760 seconds and had a
0.308913-second maximum. Direct extrapolation is 17.56 minutes at the mean and
21.21 minutes if every full-grid evaluation costs the observed maximum. A later
full run should retain streaming summaries and witnesses under 1 MiB, keep the
256 MiB cap, and use an explicit reviewed timeout. This pilot does not authorize
that run.

Fast focused tests pass: one Rust producer contract and two Python class/count
contracts. Formatting, targeted strict Clippy, and strict Python typing pass.
Maximum Rust CC/cognitive/Halstead difficulty is 7/6/19.2318; maximum Python
CC/Halstead difficulty is 12/7.18368. Every physical file remains below 500
lines. Raw pilot progress is JSON Lines, so a timeout would preserve completed
rows; this run reached a complete terminal record.

The results concern only analytical reference-evaluator arithmetic. They are
not an integrator-arithmetic study or a continuum error bound, and they exclude
raw pressure, gauge, PDE evolution, convergence, and accepted-window claims.
