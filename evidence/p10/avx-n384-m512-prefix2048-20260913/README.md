# N384/M512 minimum-prefix policy

This policy uses the unchanged source-bound full-capable binary and does not
recompile its schedule. From rest it must first produce all 32 h64 durable step
bundles through clock 2048, including observers at 512, 1024, 1536, and 2048.
That result is feasibility/incomplete-prefix evidence only and makes no endpoint
or accepted-window claim.

At durable clocks 2048 and 3072, the external controller computes the frozen
one-segment rule from the maximum integration time in the most recent eight
steps and maximum positive observer time so far. It admits exactly the next
eight h128 steps only when the resulting cost, 15% margin, and 300-second
control allowance fit before 07:00Z. Otherwise it terminates the exact owned
process group after the durable checkpoint. The full binary, every-step snapshot
policy, v3 watchdog, and independent GNU timeout remain unchanged.

The source-bound binary SHA-256 is
`85659a06a1b914d7b64feec20522eb47e876a17c626c3703d4ef59b6cb379b57`.
The prefix plan and controller are external policy. `watchdog-v2-to-v3.diff`
shows the complete watchdog change; v2 and the prior full/r3 failure evidence
remain immutable.
