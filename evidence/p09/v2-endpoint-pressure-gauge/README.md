# Endpoint pressure-mean gauge evidence

This archive preserves the successful frozen Python pressure-mean execution at
endpoint `t=1/256`, using `--panels 32`. The command was run from source
`31e99a17f97aec2ee18b26c67f8be88a0e931088` in a detached worktree with Python
3.12.3:

```sh
python -m reference.verify_pressure_mean --time 1/256 --panels 32
```

The run completed with exit code 0 in 11.67 seconds (user 11.64 s, system
0.02 s), and `/usr/bin/time -v` recorded 27,648 KiB maximum resident set size.
It evaluated 10 sequential 80/120-digit profiles with axial/radial panel
pairs 32, 64 and 128; the raw report records 77,450 pressure evaluations and
1,724,416 maximum root iterations. The preflight reserved at most 41,418,752
bytes under the 67,108,864-byte cap. `SHA256SUMS` binds this archive's files;
`module-sha256.txt` binds the measurement modules and frozen benchmark input.

This is independent global pressure-gauge quadrature evidence with empirical
refinements. It is not the installed Rust gauge allowlist, does not assign a
pressure state, and does not establish a PDE convergence window or any PDE
qualification. The raw JSON is preserved unchanged from the measurement
output.
