# Exact-v2 accepted-node reconstruction owner

`v2_run::ReconstructedRun` is a separate from-rest exact-v2 owner that retains
three actual accepted macro endpoints and their independently evaluated physical
time derivatives. The existing `v2_run::Run`, CLI profile and checkpoint format
are unchanged. No external reconstruction import is exposed.

`ReconstructedPlan::from_rest` admits the integrator, original runtime force,
history and attempt ledger together with an observer-owned doubled-grid force,
conservative-product workspace, three accepted nodes and one private proposal.
Its observer allowance is the maximum attempt count plus one: construction
independently evaluates the exact rest node, then each possible accepted endpoint
can consume one more slot. Provider work, scalar transforms and retained modal
visits are bounded separately.

The observer stages a proposed endpoint only after local integration succeeds.
The runner publishes that node only after the physical state and history commit;
rejection or refusal discards the proposal and preserves the accepted ring. The
endpoint derivative uses the observer's fresh unprojected `RunForce` and complete
conservative product, then adds the retained viscous term. It does not use an
integrator stage RHS or analytical reference state.

Once three endpoints are committed, `last_accepted_clocks` exposes their exact
clocks and `reconstruct` applies the existing quintic Hermite kernel into
caller-owned value and derivative buffers. These sampled interpolants are
diagnostic fields. This increment does not add a six-branch probe family,
off-stage residual assembly, an external reconstruction archive or a continuous
time error bound.

Focused checks are:

```sh
cargo test -p nsbu-benchmarks --test v2_reconstructed_run -- --nocapture
cargo test -p nsbu-benchmarks --test v2_reconstructed_run_allocation
cargo test -p nsbu-benchmarks --test reconstruction_observer
cargo test -p nsbu-benchmarks --test reconstruction_archive
cargo test -p nsbu-benchmarks --test owned_reconstruction
```

The N=4 startup fixture checks CM and HO against the original exact-v2 owner for
identical physical coefficient words, controller/balance history and per-attempt
work. It reconstructs the actual middle endpoint and compares its derivative to
a separately assembled original V2 force, conservative product and viscous term.
The reconstruction observer legitimately carries one extra initial-rest force
evaluation and its modal visits; the tests retain rather than hide that cost.
Rejection and advective refusal leave the accepted ring and rest state unchanged.

These checks do not qualify reconstruction error, force sampling, spatial or
temporal refinement, artifact provenance or a concentrating PDE window. P09 and
P10 remain incomplete and accepted concentrating windows remain zero.
