# NSBU Solver 0.1.0-alpha.1

[alpha-20260911-2](https://github.com/nivatechnologies/nsbu-solver/releases/tag/alpha-20260911-2)
is an immutable diagnostic research prerelease from
`88015d7681c6fb267c77d1454a3594f386d1d2ab`, published by
[workflow 34624441431](https://github.com/nivatechnologies/nsbu-solver/actions/runs/34624441431).
The preceding release tags and assets remain unchanged.

## Exact-source software checks

Both hosted suites passed on that source. Maintained Rust executable coverage
is 39,946/40,766 lines (97.99%) and 2,581/2,928 branches (88.15%), with maximum
CRAP 24.0586. Python coverage is 6,054/6,098 lines (99.28%) and 1,173/1,202
branches (97.59%), with maximum CRAP 19.9814. Required build, lint, documentation,
package, repository and static gates passed. The largest tracked Rust file has
477 physical lines; maximum cyclomatic/cognitive complexity is 21/21 and
all-node Halstead difficulty is 75.93. Mutation testing was not run in these
hosted suites; duplication and dead-code inventories remain informational.
[Full source-bound quality evidence](../../p09/hosted-alpha1-88015d7/README.md)
preserves the raw reports and exact scopes.

## Downloaded artifact checks

The Linux x86_64 GNU/glibc archive was downloaded independently and verified
against its detached checksum, all 2,224 internal checksum entries, and all
2,222 files of the exact Git source tree, including hidden paths. Its SHA-256 is:

```text
2611efa0f6373ef610de1528f0d3d094d5a3a72049e24b789f4790533dd8d4de
```

The binary reports `0.1.0-alpha.1`; its required shared libraries and maximum
GLIBC symbol version (2.35) match `SOURCE-MANIFEST.txt`. CM and HO each complete
32 steps. An HO checkpoint after step 16 resumes to exactly the uninterrupted
result except for the deliberately retained external-unverified origin label.

Both `diagnose-v2 --dry-run` and the full downloaded-binary command pass. The
full command emits all seven scheduled events at ticks 0, 7, 63, 64, 95, 127
and 128, then terminates with `UnqualifiedDiagnostic`, zero qualified windows,
and `pde_qualified: false`. [summary.json](summary.json) records the archive and
binary hashes; its `raw_sha256` inventory binds the 23 verification artifacts
under `raw/`, including the actual verification scripts and complete outputs.

## Scientific limits

This release adds the fixed startup diagnostic CLI and integrated library
consumers, alongside source-bound reference arithmetic and first-endpoint
pilot evidence. The separate N8/N12/N16 pilot remains spatially unresolved:
full-band H1 differences increase from 34.19 to 39.17. Neither passing software
checks nor the seven-event startup run establishes convergence, continuum
bounds or finite-time blow-up. P08/P09/P10 remain incomplete, with zero accepted
concentrating PDE windows. The later full-event JSON exporter, partial review
adapter and force-baseline binder are not included in this release snapshot.
