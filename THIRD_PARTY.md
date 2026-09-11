# Third-party provenance

Original NSBU Solver code and documentation use Apache-2.0. The unmodified standard license text is in [LICENSE](LICENSE). The reviewed design files are project artifacts imported with their hashes; their references do not transfer ownership of any external source.

This bootstrap includes no source code copied from `pmocz/euler-blowup-viz`, no Lean source tree, and no manuscript PDF. External papers, repositories and formalizations retain their own terms. Referencing their mathematics or linking to them is not a claim to relicense them.

## Development dependencies

| Dependency | Pinned version | Use |
|---|---|---|
| SymPy | 1.14.0 | Symbolic checks in the preserved verification script |
| mpmath | 1.3.0 | Pinned SymPy dependency and independent arithmetic reference work |

Dependencies are installed separately and are not vendored in this repository. Their own license notices remain applicable. The Cargo dependency inventories below record actual versions, licenses and native-library requirements.

The CI workflows reference the official `actions/checkout`, `actions/setup-python`
and `actions/upload-artifact` actions, pinned to exact commits. Their source is
not vendored or relicensed. The [artifact action](https://github.com/actions/upload-artifact)
preserves source-bound raw quality reports for 14 days. Official project and
installation links are given in the corresponding documentation.

## Standard license source

Apache Software Foundation: [Apache License, Version 2.0, canonical text](https://www.apache.org/licenses/LICENSE-2.0.txt). The root NOTICE applies project attribution separately; the standard license appendix remains unchanged.

## Initial quality-measurement dependencies

These optional development tools are pinned in `requirements-quality.txt` and are
not runtime dependencies or vendored source: coverage 7.10.6 (Apache-2.0), radon
6.0.1 (MIT), mando 0.7.1 (MIT), colorama 0.4.6 (BSD-3-Clause), and six 1.17.0 (MIT).
SymPy and mpmath use BSD licenses. Installed distribution notices remain applicable.

## Expanded quality environment

The tested optional environment is fully pinned in `requirements-quality.txt`.
Its installed distribution metadata, including declared licenses and license-file
paths, is recorded in [the dependency inventory](evidence/quality-reference/dependencies.json).
This includes basedpyright, its Node.js binary dependency, complexipy, pytest,
pylint, vulture, mutmut, Cosmic Ray and their transitive dependencies. They are
development tools installed separately, not solver runtime dependencies. The
inventory preserves missing or non-SPDX declarations instead of inventing license
identifiers; distribution license files remain authoritative. No dependency
source is vendored or relicensed by this project.

Pylint (GPL-2.0-or-later), astroid (LGPL-2.1-or-later) and yattag (LGPL,
as declared by its installed metadata) are separately installed development tools.
They are not linked into or shipped as part of the planned Rust library/CLI.

## Rust workspace and optional quality tools

P01 evaluated rustfft 6.4.1 (MIT OR Apache-2.0) and realfft 3.5.0 (MIT).
P03 instead uses an original bounded radix-2/3 implementation with explicitly owned
roots and scratch. The unused workspace pins were removed; neither FFT dependency
is linked or copied. The alternative spike remains historical P01 evidence.
Separately installed Rust quality tools and their licenses are declared in
[quality/rust/README.md](quality/rust/README.md); none is a runtime dependency.

P02 adds num-complex 0.4.6 (default features disabled), num-traits 0.2.19 and
build dependency autocfg 1.5.1. All declare MIT OR Apache-2.0; exact registry
checksums are in Cargo.lock and licenses in [the P02 inventory](evidence/p02/dependencies.json).
These are public Rust dependencies, with no native library requirement.

P03 adds stats_alloc 0.1.10 (MIT) as a test-only dependency for the dedicated
allocation probe. It is not linked into library or CLI release artifacts. No
dependency source is vendored; Cargo.lock records its registry checksum.


## Checkpoint artifact integrity dependencies

P09 uses SHA-256 verification in the solver artifact catalog and benchmark-owned
smooth archive, using sha2 0.10.9 with default features disabled
and its `force-soft` feature enabled. The complete resolved public dependency
graph is in [the artifact dependency inventory](evidence/p09/artifacts/dependencies.json).
Cargo.lock preserves the exact registry versions and checksums. This does not
introduce a private or native-library dependency.

The [registry license-file inventory](evidence/p09/artifacts/license-inventory.json)
preserves declared licenses and the actual notice texts and hashes. Added packages
are sha2 0.10.9, digest 0.10.7, block-buffer 0.10.4, crypto-common 0.1.7,
generic-array 0.14.7, typenum 1.20.1, version_check 0.9.5, cfg-if 1.0.4,
cpufeatures 0.2.17 and target-specific libc 0.2.189. Generic-array declares MIT;
the others offer MIT/Apache-2.0 licensing alternatives. Dependency source is
fetched separately, not copied into the project's source package. Binary release
packaging must retain applicable dependency notices.

The existing test-only stats_alloc registry archive has no separate license file;
its MIT declaration and that inventory limitation are recorded explicitly. It is
not linked into solver/CLI release artifacts. Original project LICENSE and NOTICE
are unchanged by this increment.

The physical-reduction audit adds generated project binary fixtures and no new
third-party dependency. It reuses the already declared sha2, mpmath and test-only
stats_alloc dependencies. Fixture origins and exact layouts are documented in
[the reduction guide](docs/REDUCTION_ARITHMETIC.md).
