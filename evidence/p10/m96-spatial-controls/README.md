# Cached M96 spatial controls

This archive records one completed N12 and two externally timed-out partial N16/N24 cached-force controls from frozen source `def4730b08025fdd06e7a8a0d78116aea24b6e2c`. The common profile is stated in `summary.json`. These are unqualified numerical controls; they do not establish a PDE window or convergence result.

The N12 run completed at endpoint 4096. N16 and N24 were stopped by their originally recorded external timeout at 05:49:24 UTC, retaining only coefficient snapshots explicitly declared before termination. No state was reset or continued: cached archive/resume support is unavailable and the in-process owners no longer exist.

Raw stdout is compressed under each run. N16/N24 `/usr/bin/time` files are empty because the timeout interrupted the wrapper before it could write a timing report; the empty files and launcher logs are retained. `MANIFEST.sha256` is created after final evidence files and hashes every archival file except itself and the post-check transcript. `MANIFEST.verify.txt` records the actual `sha256sum -c` result.
