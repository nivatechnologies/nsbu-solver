# Current-grid reference-arithmetic study

This bounded study measures independent 80- and 120-digit evaluation of the
four exact-v2 tracking quantities on the actual 12-cubed sample grid at elapsed
ticks 0, 64, and 128. It covers velocity (3 components), gradient (9), ordered
Hessian (27), and curl (3). Raw pressure and its gauge are excluded.

The Rust test producer emitted 5,184 rows in clock-outer, z-fast order. Before
using a row, the Python driver independently reconstructed and checked every
`binary64(i / 12)` argument word, centered coordinate word, elapsed-time word,
input class, row ordinal, and row identity. It rejected duplicate rows,
nonfinite Rust outputs, nonzero shortcut outputs, and any multiprecision result
whose four vectors were not finite and exactly 42 components long. The frozen
case SHA-256 was
`e1236f7b3c51537acd17381402ca420ba7872a7b9dbc64b2f0d9d5108a468f7e`.

The complete guarded run reached terminal row 5,184 with 1,030 active rows and
4,154 legitimate startup/exterior shortcuts. It performed 4,120 independent
multiprecision jet evaluations and 10,368 Rust reference-evaluator calls: the
producer evaluates both the original and centered coordinates for each row.
All word validation, multiprecision vector validation, hashing, reductions,
and terminal formatting ran inside an explicit 120-digit context.

Internal time was 1,684.93 seconds and measured wall time was 28:05.07, below
the fixed 45-minute timeout. `RLIMIT_AS` was 256 MiB independently for the
Python process and Rust producer. `/usr/bin/time` reported 27,648 KiB maximum
RSS, which is not an aggregate of simultaneous parent and child RSS. The driver
streams rows, hashes, extrema, and witnesses without retaining a full results
map; the raw JSON Lines output is 93,290 bytes.

The largest 80-to-120-digit absolute change was
`6.26145635811771475714544603742e-83`, in a word-bound ordered-Hessian
component. This passes the retained `1e-60` diagnostic. That diagnostic checks
the study's chosen arithmetic precision; it is not a binary64 acceptance
threshold. The largest exact-rational to exact-binary-word input effect was
`2.84483867746364703261604765007e-17`, and the largest Rust-binary64 to
120-digit word-bound discrepancy was
`1.05196815305015347551483874511e-17`; both occurred in ordered-Hessian
components. `summary.json` and the terminal raw record retain per-quantity
maxima and exact witnesses.

The terminal field `rms_scaled` is a componentwise RMS of scaled component
errors over points times the number of components in that quantity. It is not
the production tracking norm, whose vector or Frobenius norm is formed per
point and then averaged over points. The maxima and witnesses are direct
component measurements and do not require this normalization interpretation.

An earlier run was stopped at row 2,176 after review found that its reductions
resumed at mpmath's default precision. It has no terminal record and supports no
result or completion claim. Its raw stream and a separate invalidity record are
retained so the restart is auditable.

Focused validation passes 13 Python tests and two Rust producer tests. Python
coverage is 318/357 executable lines and 79/96 branches (87.64% and 82.29%);
the Rust producer test file covers 93/95 lines and 10/10 branches (97.89% and
100%). Maximum CRAP is 19.9815 for the new Python driver and 7 for the Rust
producer. Maximum Python CC is 15; Rust CC/cognitive complexity is 7/6.
Whole-file/all-node Halstead difficulty is 12.5 for Python and 44.7387 for
Rust. All maintained files are below 500 physical lines. Formatting, targeted
strict Clippy, strict Python typing, and final workspace Rustdoc pass.

These results measure empirical analytical reference-evaluator arithmetic.
They are not an enclosure, integrator-arithmetic study, continuum bound,
trajectory or convergence result, or accepted-window claim. The complete raw
record, invalid partial, commands, timings, coverage, metrics, CRAP reports,
and quality logs are compressed under `raw/`; source and artifact hash
manifests bind them to this increment.
