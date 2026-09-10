# nsbu-cli

Public command-line entry point for NSBU Solver. `nsbu smooth` runs the bounded built-in
`CyclicSine` numerical diagnostic with either Cox--Matthews (`cm`) or
Hochbruck--Ostermann (`ho`). `--dry-run` validates the exact configuration and resource ledger
through `SmoothPlan` before numerical state is allocated. `nsbu smooth --checkpoint PATH
--checkpoint-after N` saves after exactly `N` accepted steps without changing the profile's
original endpoint or configuration. `nsbu resume --checkpoint PATH` requires matching supplied
profile options and continues only as an `external_unverified` diagnostic run.

The command reports JSON. Exact tick counts are decimal strings, and the report always marks the
profile as not PDE-qualified. Checkpoints are atomically published without replacing an existing
path, and bounded file reads reject oversized or changing files. Invalid command syntax uses exit
code 2; resource, checkpoint, or numerical refusals and incomplete runs use exit code 1.

Part of NSBU Solver, licensed under Apache-2.0. No private Niva dependency.
