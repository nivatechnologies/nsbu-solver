# Alpha releases

The alpha workflow produces dated, immutable Linux x86_64 CLI snapshots from
the public repository. It is a distribution convenience for the bounded
runtime and its diagnostics. An alpha is explicitly **diagnostic-only and
scientifically unqualified**: it does not establish PDE qualification, a
concentrating solution, a finite-time singularity, or literal reproduction of
the original research target.

Each archive contains the `nsbu` binary, `LICENSE`, `NOTICE`, `README.md`, the
`docs/` tree, `project-status.json`, `IMPLEMENTATION_PLAN.md`, a
`SOURCE-MANIFEST.txt`, an in-archive `SHA256SUMS` file, and a detached archive
`.sha256` file. The manifest records the full source commit, snapshot tag,
required CI workflow identifiers, target triple, Cargo workspace version, and
scientific status.
The snapshot tag identifies the source and is independent of the Cargo crate
version; the current workspace version remains `0.1.0-alpha.0` until a
deliberate package-version change.

The first snapshot is started manually from the `main` branch and receives an
`alpha-first-YYYYMMDD` tag. Daily scheduled runs receive `alpha-YYYYMMDD`.
The workflow refuses an existing tag or release, refuses a source commit that
is unchanged since the prior alpha, and refuses a duplicate first snapshot.
It does not overwrite releases or force-push tags. A daily run that falls on a
date whose tag already exists is a no-op rather than replacing it.

## Readiness gate

The workflow is intentionally disabled for release until `project-status.json`
contains the tracked, reviewed readiness record at the root:

```json
"alpha_release": {
  "ready": true,
  "status": "runtime-complete",
  "cadence": "daily",
  "accepted_concentrating_windows": 0,
  "scientific_qualification_separate": true
}
```

The readiness record is a project-level release switch; scientific
qualification remains separate and `accepted_concentrating_windows: 0` is
valid for this diagnostic alpha. The workflow also requires a successful
source-matched run of both existing
`checks.yml` (Python/repository checks) and `rust.yml` (Rust package and quality
checks) for that exact commit. Those checks remain the single source of truth;
the release workflow does not duplicate their long suites. Root project
status ownership should add this record only after the runtime checklist,
checkpoint/resume walkthrough, package checks, and hosted CI evidence support
the exact commit.

The build uses the repository-pinned Rust `1.94.0` toolchain and performs
`nsbu --version` and `nsbu --help` smoke checks before creating a tag. Inside an
extracted archive, run `cd nsbu-solver-*-x86_64-unknown-linux-gnu && sha256sum
-c SHA256SUMS` to verify the binary and manifest.

The workflow requests `actions: read` to inspect those source-matched workflow
runs and `contents: write` only because GitHub needs it to create the immutable
tag and prerelease. The token is used by the job; no registry publish or
external service is involved. A scheduled run cannot release while the gate is
absent or false. A manually dispatched run has the same gate and must run on
`main`.

Release notes include the commit range from the previous alpha and repeat the
diagnostic/unqualified status. GitHub's release and tag APIs may still reject
a run because of repository policy, branch protection, disabled Actions, or
token restrictions; those are hosted configuration limits, not evidence that
the runtime is ready.
