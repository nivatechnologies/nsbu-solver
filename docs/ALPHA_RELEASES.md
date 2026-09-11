# Alpha releases

The alpha workflow produces dated, immutable Linux x86_64 GNU/glibc CLI snapshots from
the public repository. It is a distribution convenience for the bounded
runtime and its diagnostics. An alpha is explicitly **diagnostic-only and
scientifically unqualified**: it does not establish PDE qualification, a
concentrating solution, a finite-time singularity, or literal reproduction of
the original research target.

Each archive contains the `nsbu` binary and the complete tracked source tree,
including hidden build inputs, licenses, documentation and verification evidence.
This preserves local documentation links and supports inspection and rebuilding.
`SOURCE-MANIFEST.txt` records the full source commit, snapshot tag, required CI
workflows, target triple, Cargo version and scientific status. `SHA256SUMS` covers
every bundled source file, binary and manifest; a detached `.sha256` verifies the
complete compressed archive.
The snapshot tag identifies the source and is independent of the Cargo crate
version. The workspace now targets `0.1.0-alpha.1`; the existing
`alpha-20260911` binary remains version `0.1.0-alpha.0`. A workspace version
change does not itself publish a release.

The first snapshot is started manually from the `main` branch and receives an
`alpha-first-YYYYMMDD` tag. Daily scheduled runs receive `alpha-YYYYMMDD`.
When a same-day daily snapshot is needed after additional manual validation,
dispatch the workflow with `release_kind=daily` and a positive decimal
`snapshot_revision` such as `2`; it selects `alpha-YYYYMMDD-2`. The revision
input is manual-only. It is rejected for `first-alpha`, schedules, non-decimal
values, and non-positive values. An empty revision keeps the standard
`alpha-YYYYMMDD` tag.
The workflow refuses an existing tag or release, refuses a source commit that
is unchanged since the prior alpha, and refuses a duplicate first snapshot.
It does not overwrite releases or force-push tags. A daily run whose selected
tag already exists is a no-op rather than replacing it.

## Binary compatibility

The binary is built and smoke-tested on the pinned `ubuntu-24.04` runner with
Rust 1.94.0, for `x86_64-unknown-linux-gnu`. It dynamically links system libraries;
it is not a musl/Alpine binary. `SOURCE-MANIFEST.txt` records the build glibc,
highest required GLIBC symbol version and needed libraries from the actual
artifact. These are inspectable compatibility requirements, not a portability
test on every distribution. Other Linux distributions, macOS and Windows need
a source build unless a separately tested artifact is provided.

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
status records local readiness after the runtime checklist, checkpoint/resume
walkthrough, package checks, and required quality gates pass. Publication then
waits for successful hosted checks for the exact commit containing that flag.

The build uses the repository-pinned Rust `1.94.0` toolchain and performs
`nsbu --version` and `nsbu --help` smoke checks before creating a tag. Inside an
extracted archive, run `cd nsbu-solver-*-x86_64-unknown-linux-gnu && sha256sum
-c SHA256SUMS` to verify every bundled source file, binary and manifest.

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
