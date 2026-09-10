# Automated quality-checker CLI contract

The revised-policy Python workflow at commit `3b88195` passed its 159 tests but
failed CRAP: automated coverage did not execute the checker's command-line path.
The earlier local coverage included a separate manual invocation. That mismatch
is retained in the preceding hosted report and failure log here.

Four additional tests now invoke the real CLI as subprocesses. They verify a
passing Python report, a Rust report that fails the CRAP threshold, malformed or
empty evidence, and invalid command arguments. They check output and exit codes.

The final complete suite passed **163 tests** in 220.03 seconds. Coverage is
3362/3366 executable lines (99.88%) and 607/612 branches (99.18%); maximum CRAP is
15. Strict typing, complexity and repository checks passed. The normal pytest
invocation now supplies this coverage without an additional manual CLI run.

The raw measurements and source hashes are in [summary.json](summary.json).
Hosted verification of this fix remains pending. An intermediate local suite
failed because an unfinished checkpoint guide linked to unintegrated source;
the guide was moved back to ignored working storage before the final clean run.
Numerical sources and acceptance criteria did not change in this fix.
