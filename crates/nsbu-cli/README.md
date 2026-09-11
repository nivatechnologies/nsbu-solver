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

Try the installed binary with:

```sh
nsbu v2 --help
nsbu resume-v2 --help
nsbu v2 --dry-run
```

JSON reports retain exact times, all charged work, the last accepted balance
terms, pending Simpson/integral state, and `origin_status`. A new run reports
`internal_from_rest`; a restored checkpoint reports `external_unverified`.
Terminal refusals and incomplete trajectories exit nonzero. Successful output
is diagnostic-only and does not establish PDE qualification, convergence, or a
concentrating window. The crate is part of NSBU Solver under Apache-2.0 and has
no private Niva dependency.
