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
