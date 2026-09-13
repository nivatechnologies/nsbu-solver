# Fresh M512 endpoint r6 execution envelope

This is a prepared launch envelope only. It preserves the M512 scientific plan,
binary, source, and rest initial state identified in `frozen-execution-envelope.json`.
It creates neither a remote stage nor a solver process during review.

The wrapper refuses unless `NSBU_LAUNCH_N384_M512_R6=1`; this refusal is the
local dry-run control. A future reviewed staging action must copy the immutable
binary, scientific plan, watchdog, envelope, and wrapper into the stated fresh
remote stage, verify the recorded hashes, and leave r4 and r5 unchanged.

`./launch-reviewed.sh --self-test-startup-cleanup` is a no-solver safety test.
It starts a private `setsid sleep`, captures its PID/PGID/starttime/command-line
identity, exercises the pre-watchdog TERM cleanup path, and confirms that the
owned leader has exited. It never uses a broad process search or starts the
scientific binary.

`./launch-reviewed.sh --self-test-signal-cleanup` starts the same private
synthetic leader in a child wrapper, sends that wrapper TERM, and verifies from
the parent that the signal trap caused bounded cleanup of the exact owned group.
If an actual launched leader cannot be identity-bound, the launcher deliberately
does not signal an unverified PID or process group; GNU timeout remains the
deadline bound in that narrow startup failure case.
