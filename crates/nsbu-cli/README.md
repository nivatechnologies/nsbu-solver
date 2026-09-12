# nsbu-cli

The public `nsbu` executable provides bounded smooth and exact-v2 diagnostic
runs. `nsbu v2` starts the reviewed `similarity-mms-v2` trajectory from exact
rest; `nsbu resume-v2 --checkpoint PATH` continues a same-profile checkpoint
after bounded input and decoded-size preflight. Both CM and HO are supported.

The v2 defaults are grid `N=4`, force grid `M=4`, step `128` ticks, quantum
`2^-20`, endpoint `4096` ticks (`T1=1/128`), maximum `32` attempts, absolute
tolerances `[1e-5, 1e-4]`, relative tolerances `[1e-5, 1e-5]`, and advective
guard `0.3`. `--workers N` selects a finite persistent-worker allowance;
`--workers 0` uses the serial provider. Resource preflight includes the maximum
decoded checkpoint reservation and input buffer before construction.

The default v2 profile evaluates the original force directly. `--cache-force` explicitly
selects the attempt-local original-force cache; it reports that integration policy and
current-attempt cache counters separately from the aggregate charged-work ledger. Cached
runs cannot write or resume archives. For example:

```sh
nsbu v2 --cache-force --method cm --endpoint-ticks 256 --attempts 4
```

Try the installed binary with:

```sh
nsbu v2 --help
nsbu resume-v2 --help
nsbu v2 --dry-run
nsbu diagnose-v2 --dry-run
nsbu diagnose-v2
```

`diagnose-v2` has no numerical profile options. It admits the fixed N=4/8/12
startup profile and complete clocks `[0,7,63,64,95,127,128]`. Its dry run stops
before diagnostic-driver allocation. The actual command emits concise raw
accepted/residual summaries labeled `UnqualifiedDiagnostic`, lists every
missing evidence channel and reports zero qualified windows. Applications can
use the benchmark library API when they need the complete retained report
structures rather than CLI summaries.

JSON reports retain exact times, all charged work, the last accepted balance
terms, pending Simpson/integral state, and `origin_status`. A new run reports
`internal_from_rest`; a restored checkpoint reports `external_unverified`.
Terminal refusals and incomplete trajectories exit nonzero. Successful output
is diagnostic-only and does not establish PDE qualification, convergence, or a
concentrating window. The crate is part of NSBU Solver under Apache-2.0 and has
no private Niva dependency.
