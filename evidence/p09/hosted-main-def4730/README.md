# Main source quality verification

Both hosted workflows passed on `def4730b08025fdd06e7a8a0d78116aea24b6e2c` on main. [Summary](summary.json) recomputes separate line/branch coverage, static maxima and CRAP from the retained raw reports. Every maintained line count comes from the same Git source. This establishes the release quality gate, not PDE convergence or validation of later development commits.

The gzip artifacts preserve original bytes when decompressed. Python coverage.py combines lines and branches in its percent_covered field; the summary computes each separately.
