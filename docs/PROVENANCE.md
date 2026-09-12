# Provenance and adopted decisions

The public project name is **NSBU Solver**, the repository is `nivatechnologies/nsbu-solver`, and the license for original project material is **Apache-2.0**. Earlier reviewed documents use the working name `navier-runtime`. They are retained byte-for-byte so their recorded hashes and numerical review remain meaningful.

The active [implementation plan](../IMPLEMENTATION_PLAN.md) supersedes the historical plan's naming and proposed dual-license statement. This is an explicit project-governance overlay, not a mathematical design change. Historical release aspirations do not override the scoped current decisions in [project-status.json](../project-status.json) and [scientific scope](SCIENTIFIC_SCOPE.md).

## Frozen baseline

| File | SHA-256 |
|---|---|
| `docs/design/COMPLETE_DESIGN.md` | `fabca082cf73fee5f64c7f67800308b5115936bbec04838100a0d2ee425b81d9` |
| `docs/design/similarity-mms-v2.json` | `e1236f7b3c51537acd17381402ca420ba7872a7b9dbc64b2f0d9d5108a468f7e` |

Both are revision 0.7. The companion assessment, construction ledger, historical plan and checks are revision 0.8. The import includes the 12 public review components, their original manifest and the assembled packet. The packet is a historical review artifact, not instructions to contributors or an executable workflow.

### Exact imported filenames

All 14 files below live under `docs/design/` and retain their original bytes. The first 12 are the components recorded in the original manifest.

| File | Role |
|---|---|
| `COMPLETE_DESIGN.md` | Adopted runtime specification, revision 0.7 |
| `similarity-mms-v2.json` | Exact manufactured-case specification, revision 0.7 |
| `ADVERSARIAL_REVIEW.md` | Review disposition |
| `CONSTRUCTION_LEDGER.md` | Source-construction obligations and admission scope |
| `IMPLEMENTATION_PLAN.md` | Historical detailed work packages and numerical fixtures |
| `SOURCE_FEASIBILITY.md` | Scoped source-feasibility assessment |
| `source-feasibility-policy.json` | Machine-readable policy and evidence limits |
| `navier-runtime-adversarial-review-prompt.md` | Historical complete review prompt |
| `navier-runtime-design.md` | Historical runtime entry point |
| `navier-runtime-construction-design.md` | Historical construction entry point |
| `navier-runtime-verification.py` | Preserved executable mathematical verification script |
| `navier-runtime-verification-results.json` | Preserved earlier execution report |
| `navier-runtime-review-manifest.json` | Original component names, sizes, revisions and SHA-256 values |
| `navier-runtime-review-packet.md` | Complete assembled review packet |

The original manifest SHA-256 is `4393e8766fe00cd4f3d92452479ac6fe1680d08319b05c9b7f1e4a3908fd2297`. The assembled packet SHA-256 is `efe01c961f171eb4b8adee1286ae4051b7ce2af2abd3eb6334c5211f90254af8`.

`tools/check_repository.py` is the new bootstrap checker. `tools/verify_design.py` invokes the preserved `docs/design/navier-runtime-verification.py`; it does not reconstruct or replace that script. Guard tests are in `tools/tests/test_bootstrap_tools.py`. The workflow is `.github/workflows/checks.yml`; the leading dot is part of the directory name and must survive file transfer.

The convenient `benchmarks/similarity-mms-v2.json` copy must remain byte-identical to the frozen case. The canonical mathematical problem hash is `ba81b7709e68cb118d3fc60c1d9bbcd27f424358b121ed4be08660ab8b88210f`. Its scope is the exact syntactic definition, not mathematical-equivalence detection.

Run `python tools/check_repository.py` to verify component hashes, the packet hash, the two frozen baseline hashes and the benchmark's canonical problem identity. The original package manifest records a separate historical ZIP; that ZIP is not needed to verify the imported component files.

The imported code and documents are project artifacts. No manuscript PDF, third-party source repository, private adapter, personal workspace data or private integration specification is included. Full formal proof verification and source-PDF/repository byte provenance are not established by these component hashes.

## New execution evidence

The preserved `navier-runtime-verification-results.json` is earlier execution evidence. New runs write to `work/` by default or to an explicitly reviewed evidence path. A successful bootstrap rerun must match the preserved scientific report before adding `bootstrap_execution` metadata. An evidence record must name its input hashes, command, interpreter/dependency versions, actual status and unperformed work. Never overwrite the preserved report to make a baseline check pass.

The bootstrap's Git history describes repository initialization only. It must not imply that numerical implementation milestones are complete. Hosted CI and remote publication are verified separately from local checks.

The public `nsbu-benchmarks` package also embeds a byte-identical copy at
`crates/nsbu-benchmarks/data/similarity-mms-v2.json`. The Rust workflow compares it
with the canonical benchmark before packaging. Both paths disable line-ending
conversion. The embedded artifact's historical status text remains unchanged;
current capability is recorded separately in `project-status.json`.

The generated `fixtures/reference/derived-n4-cm.json.gz` is a deterministic gzip
copy of the public `smooth_derived` Rust example's actual N=4 CM export. It is a
regression input, not an independent oracle or a frozen review artifact. The
independent Python derived-field tests recompute its complete fields with direct
high-precision sums and separately test analytic Taylor–Green pressure and all
ordered derivatives. Regeneration uses the exact from-rest schedule and raw force
inputs documented in [derived-field arithmetic](DERIVED_ARITHMETIC.md); changes
must retain the original and replacement input hashes in execution evidence.

## Archived source revisions for evidence reproduction

Some source-bound evidence records revisions from focused work that was not
merged into the public `main` history. The archive-only branch
[`codex/evidence-sources-20260912`](https://github.com/nivatechnologies/nsbu-solver/tree/codex/evidence-sources-20260912)
retains those ten revisions as additional ancestry. Its archive commit is
`200c5d4c8d02caf18561d4b8c1e2026e8168e924`; its tree is exactly the
`def4730b08025fdd06e7a8a0d78116aea24b6e2c` tree. The [source-history record](../evidence/p09/source-history-20260912/summary.json)
contains the full revision list and verification values.

The archive commit is for source retrieval and provenance. Its extra ancestry
does not merge implementation into `main`, publish a release, or establish a
new quality result. The evidence retains each report's own source and scope;
P08/P09/P10 qualification and accepted concentrating windows remain separate.

A fresh public clone can retrieve the archive ancestry and inspect the exact
evidence tree as follows:

```sh
git clone https://github.com/nivatechnologies/nsbu-solver.git
cd nsbu-solver
git fetch --no-tags origin \
  refs/heads/codex/evidence-sources-20260912:refs/remotes/origin/codex/evidence-sources-20260912
git checkout --detach refs/remotes/origin/codex/evidence-sources-20260912
test "$(git rev-parse HEAD^{tree})" = \
  "$(git rev-parse def4730b08025fdd06e7a8a0d78116aea24b6e2c^{tree})"
git show HEAD:evidence/p09/v2-attempt-force-cache/summary.json
SOURCE_SHA=9836cfc9f3b13a6f3df39865bd016d17e6bc1e85
git merge-base --is-ancestor "$SOURCE_SHA" HEAD
git checkout --detach "$SOURCE_SHA"
git show --stat --oneline "$SOURCE_SHA"
git show "$SOURCE_SHA":crates/nsbu-benchmarks/src/runtime_force/attempt_cache.rs | sed -n '1,40p'
```

The archive tree is deliberately the `def4730b` tree, so the explicit `git show`
above inspects an evidence file present in that exact tree before checking out a
retained source revision. The source checkout then shows the actual focused
implementation file associated with `SOURCE_SHA`.

A source ZIP created from the exact `def4730b` revision reproduces the same
tracked files and evidence tree, but it cannot carry Git parent ancestry. Use
the archive branch when checking the retained source revisions; use the ZIP
when only the exact source tree is required.
