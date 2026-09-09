# Third-party provenance

Original NSBU Solver code and documentation use Apache-2.0. The unmodified standard license text is in [LICENSE](LICENSE). The reviewed design files are project artifacts imported with their hashes; their references do not transfer ownership of any external source.

This bootstrap includes no source code copied from `pmocz/euler-blowup-viz`, no Lean source tree, and no manuscript PDF. External papers, repositories and formalizations retain their own terms. Referencing their mathematics or linking to them is not a claim to relicense them.

## Development dependencies

| Dependency | Pinned version | Use |
|---|---|---|
| SymPy | 1.14.0 | Symbolic checks in the preserved verification script |
| mpmath | 1.3.0 | Pinned SymPy dependency and independent arithmetic reference work |

Dependencies are installed separately and are not vendored in this repository. Their own license notices remain applicable. The future Cargo dependency selection must record actual versions, licenses and any native library requirements before a public package is released.

The CI workflow references the official `actions/checkout` and `actions/setup-python` actions. Their source is not vendored or relicensed. Official project and installation links are given in the corresponding documentation.

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
