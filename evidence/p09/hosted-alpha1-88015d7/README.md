# Hosted alpha.1 quality evidence

This directory preserves the complete downloaded Rust and Python hosted quality
artifacts for alpha.1 source `88015d7681c6fb267c77d1454a3594f386d1d2ab`.
The source markers embedded in both uploaded artifact sets equal that revision.
The Git tree identity is recorded separately. GitHub Actions runs 34613195748
(Rust) and 34613195894 (Python) both completed successfully.

The complete maintained Rust scope covered 39,946/40,766 executable lines
(97.9885%) and 2,581/2,928 instrumented branches (88.1489%). Maximum CRAP was
24.05859375 across 3,595 scored functions. Independent traversal of every
nested metrics node found per-function cyclomatic and cognitive maxima of 21,
an all-node Halstead difficulty maximum of 75.9295154185022, an RCA unit-node `ploc` maximum of 462, and a true maximum file size of 477 physical lines. The file-size result uses `len(git show REV:PATH bytes.splitlines())` over every one of the 453 hosted Rust source-list paths; it does not use RCA `ploc`.

The Rust coverage stage reported 157 terminal test groups totaling 510 passed
and zero failed tests. The earlier public-workspace stage separately reported
one passing doctest. These stages are kept separate to avoid presenting repeated
or differently instrumented execution as one test population.

The complete maintained Python scope covered 6,054/6,098 executable lines
(99.2785%) and 1,173/1,202 branches (97.5874%). Coverage.py displayed 99% for
its combined metric. Maximum CRAP was 19.9814453125 across 956 scored functions.
The retained static reports have a cyclomatic maximum of 16 and Halstead
difficulty maximum of 13.826086956521738. The workflow passed 41 tool unittests
and 236 coverage pytest tests with zero failures.

All downloaded quality reports, source-file inventories and complete workflow
logs are stored as deterministic gzip streams in `raw/`. `artifact-sha256.json`
binds every retained artifact. The duplication, lexical/dead-code and source
inventories remain available for review. The Rust duplication tool reported
5.8% under its informational exit-status policy; it was not an acceptance gate.

Mutation testing was not run in these workflows and is recorded as
informationally absent. This is hosted software-quality evidence; it adds no
numerical study, qualification, or accepted-window claim.

The repository packaging/integrity checker initially failed closed because a concurrent root release edit linked `evidence/runtime-alpha/alpha-20260911-2/README.md` before that target existed. After the release evidence target appeared, the final check passed: 2,273 public files, 140 Markdown files and 693 local links were checked. Both raw results are retained; this task did not alter the release files.
