# Daily alpha · 11 September 2026

[alpha-20260911](https://github.com/nivatechnologies/nsbu-solver/releases/tag/alpha-20260911)
is published from `6170341c42a64348c7d99bdfd4fc3653454f22a8`. It adds six
independent exact-v2 comparison trajectories, global physical derivative
comparisons, and the optional reduced-coordinate force evaluator/provider.
The default CLI and checkpoint path retain the original force provider.

Both hosted workflows passed on that exact revision. Rust executed 451 harness
tests, 13 isolated allocation probes and one compiled documentation example.
Executable-line coverage is 31,461/31,914 (98.58%); branch coverage is 2,174/2,446
(88.88%). Maximum cyclomatic complexity is 21, cognitive complexity 18,
function/file Halstead difficulty 75.8956, physical file size 477 lines and
CRAP 24.33594. All required thresholds pass. Dead-code, duplication and mutation
findings retain their documented informational status.

Python passed 220 tests, including the bootstrap checks. Its executable-line
coverage is 5,512/5,522 (99.82%) and branch coverage is 1,085/1,098 (98.82%).
Maximum cognitive complexity is 19. The raw coverage.py combined percentage is
99.65%; it combines executable lines and branches. Earlier quality summaries
labelled that combined value as line coverage. This report computes the two
percentages separately from the preserved numerators; neither interpretation
changes the passing gate.

The downloaded Linux x86_64 archive is 84,106,578 bytes, with SHA-256:

```text
ecb07b6b9e7828d0fc125c3a20c9d61be2db8e83defb6074bbb6bb2752abd857
```

All 1,829 internal file checksums and repository checks passed after extraction.
The installed CM and HO commands each completed 32 committed steps to 1/256.
An HO checkpoint after step 16 resumed to the same complete result as the
uninterrupted run, except for the intentionally retained external-unverified
origin. The artifact was built on Ubuntu 24.04 and requires GLIBC symbols up to
2.35; the source manifest names its target and shared-library dependencies.

[summary.json](summary.json) records the publication and download result;
[hosted-quality.json](hosted-quality.json) records detailed metric scopes.
The [raw archive](raw-artifacts.tar.gz) contains source-bound hosted reports,
workflow logs and the exact download-verification script. Its members are
individually hashed in [raw-sha256.json](raw-sha256.json). The script retains
historical local paths, which must be adjusted when reproducing it elsewhere.

Daily publication is scheduled for 02:17 UTC when new source changes pass both
hosted workflows. Existing release tags are immutable. This prerelease remains
a diagnostic runtime with zero accepted PDE windows; P08/P09/P10 scientific
exits remain open. Neither the later reduced-provider trajectory tests nor the
exact-v2 pressure consumer under development is included in this release.
