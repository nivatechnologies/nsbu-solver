# nsbu-cli

Public command-line entry point for NSBU Solver. `nsbu smooth` runs the bounded built-in
`CyclicSine` numerical diagnostic with either Cox--Matthews (`cm`) or
Hochbruck--Ostermann (`ho`). `--dry-run` validates the exact configuration and resource ledger
through `SmoothPlan` before numerical state is allocated.

The command reports JSON. Exact tick counts are decimal strings, and the report always marks the
profile as not PDE-qualified. It does not provide checkpoint commands. Invalid command syntax
uses exit code 2; resource or numerical refusals and incomplete runs use exit code 1.

Part of NSBU Solver, licensed under Apache-2.0. No private Niva dependency.
