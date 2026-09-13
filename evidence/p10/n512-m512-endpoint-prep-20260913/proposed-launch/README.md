# Proposed N512/M512 launch template

This directory is a reviewable template only. It has not launched the solver,
created an endpoint run directory, or copied an archive. The reviewed
`frozen-plan.json` remains byte-for-byte unchanged.

The template requires explicit launch opt-in, the exact resolved bundle path,
an absolute archive parent, and a caller-supplied numeric absolute deadline
with at least 66,672 seconds remaining. It binds the tested binary, immutable
plan, preflight receipt, and reviewed v3 process-identity watchdog by SHA-256.
It applies the exact 241,937,824,240-byte memory floor and
189,586,276,352-byte source and archive disk floors.

After process construction it captures PID, process group, `/proc` start time,
and command-line hash before attaching the v3 watchdog. The first-step gate
requires clock 64, 12 timed RHS calls, seven cache hits, five misses, zero
steady allocations, integration time no greater than 1,200 seconds, and at
least 65,410 seconds still available. A failed gate signals only the verified
owned process group.

After a zero-status endpoint exit, the template checks the exact 48-step clock
sequence, committed attempt counters, coefficient sizes, all state payloads,
the eight offline-observer markers, terminal marker, and false qualification.
It streams the output into a create-new partial archive, compares sorted source
and destination SHA-256 inventories, syncs the destination, and only then
renames the archive.

The prepared binary hash is retained for provenance but its profile identity
still advertises the legacy v2 external-stop label. The normal launch path
therefore has an unconditional identity-mismatch refusal after deadline
validation. Before any real launch, rebuild the unchanged numerical source
with an accurate v3 harness identity and refreeze the binary, preflight, plan,
watchdog, and launcher hashes. Supplying a deadline or opt-in does not bypass
this refusal and does not constitute authorization.

The only executed controls are shell syntax validation, default no-opt-in
refusal, numeric/expired/short deadline refusals, exact-minimum deadline
acceptance, and an identity-bound fake `sleep` child terminated by the v3
watchdog. These controls allocate no numerical state.

## Refrozen v3 bundle

The `v3-*` files and `v3-launch-plan.json` are the later, still-unexecuted
replacement for the original proposed v2-bound packet above. The N512-only
harness identity was rebuilt with
`external_stop=pgid-watchdog-v3-confirmed-identity-absolute-deadline` at harness
and `RUN_SOURCE` commit `e25f3816f83c6a7c07202cac2878f58ace460511`.
The original frozen trajectory plan, v2 binary, and preparation receipts remain
unchanged and are referenced by the new plan.

The v3 launcher requires a future caller-supplied absolute deadline and explicit
opt-in. It acquires a stable post-`setsid` PID, PGID, start-time, and command-line
identity before treating a process group as owned. Failure cleanup gives a
confirmed owner at most 60 seconds after TERM, then sends KILL and reaps it. The
same absolute deadline watchdog covers the solver and the local archive helper.
The archive is a create-new local Sulaco copy with source/destination inventories,
`sync`, and atomic final rename; this packet does not claim a remote durable
transfer to baccus. `v3-launch-receipt.json` binds the launcher itself because a
launcher cannot contain its own SHA-256 without a circular value.
