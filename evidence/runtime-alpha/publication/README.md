# First alpha publication

[NSBU Solver alpha-first-20260911](https://github.com/nivatechnologies/nsbu-solver/releases/tag/alpha-first-20260911)
is a public GitHub prerelease of source commit
`63f9a145c8e3b0168e7f8896f0f92f3217cdf57f`. Its immutable annotated tag was
created by the release workflow after exact-source Rust and Python CI passed.
No existing tag, branch or release was overwritten or force-pushed.

[summary.json](summary.json) records publication and the shipped artifact check.
The hosted Rust run passed all required gates with 425 harness tests, nine
allocation executables and one compiled public documentation example. The
hosted Python run passed all 220 tests, required metrics, bootstrap checks and
preserved mathematical verification. Raw logs and their SHA-256 inventory are
retained alongside the workflow records.

The downloaded Linux x86_64 GNU/glibc archive is 77,974,864 bytes with SHA-256
`9dc067b6591eba79effe76e4e28a8c269a33b91736d9edc1c05a426ea5700b11`.
Its detached checksum, all 1,660 internal checksums, and repository/frozen-input
checks pass. The shipped binary completes default 32-step CM and HO runs.
Saving HO after 16 steps and resuming to 32 reproduces the uninterrupted JSON
apart from the required external-unverified origin. Actual reports are retained.
A separate public clone with the Git credential helper disabled also passes
repository checks at the same source commit.

The source manifest records Ubuntu 24.04, build glibc 2.39, maximum required
symbol GLIBC_2.35, and the actual dynamic libraries. This is a tested Linux
artifact, not a claim of portability across every distribution or processor.

The daily release workflow is scheduled for 02:17 UTC, requires successful
exact-source CI and new changes, and does not replace prior snapshots. The
first release's notes were expanded after workflow publication with executable
quickstart commands and measured limits; its immutable source tag and archive
were not changed.

This verifies runtime distribution and behavior. There are zero accepted
concentrating PDE windows; P08/P09/P10 qualification remains incomplete.
