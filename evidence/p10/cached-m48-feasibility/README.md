# Cached M48 feasibility run

This archive preserves one completed cached-force diagnostic run from frozen numerical source `5fa65efbad0cc340efdf126278feaf579e0017fa`. It is an unqualified numerical feasibility study and does not establish a PDE window or convergence qualification.

`raw/full.stdout.gz` and `raw/full.time.gz` are the complete declared run output and `/usr/bin/time -v` record. The 30 files in `forensics/` are selected strictly from `coeff_forensics ... path=` declarations in that full stdout. The shared ignored working directory also held an earlier endpoint-128 walkthrough; its clock-64 and clock-128 files are deliberately absent here.

The harness is retained with its launch/preflight records. Cached v1 checkpoint operations were unsupported and no archive/restart was attempted. `summary.json` is path-free metadata; `MANIFEST.sha256` hashes every other evidence file in this directory.
