# Third-party provenance

Original NSBU Solver code and documentation use Apache-2.0. The unmodified standard license text is in [LICENSE](LICENSE). The reviewed design files are project artifacts imported with their hashes; their references do not transfer ownership of any external source.

This bootstrap includes no source code copied from `pmocz/euler-blowup-viz`, no Lean source tree, and no manuscript PDF. External papers, repositories and formalizations retain their own terms. Referencing their mathematics or linking to them is not a claim to relicense them.

## Development dependencies

| Dependency | Pinned version | Use |
|---|---|---|
| SymPy | 1.14.0 | Symbolic checks in the preserved verification script |
| mpmath | 1.3.0 | Pinned SymPy dependency and future independent arithmetic reference work |

Dependencies are installed separately and are not vendored in this repository. Their own license notices remain applicable. The future Cargo dependency selection must record actual versions, licenses and any native library requirements before a public package is released.

The CI workflow references the official `actions/checkout` and `actions/setup-python` actions. Their source is not vendored or relicensed. Official project and installation links are given in the corresponding documentation.

## Standard license source

Apache Software Foundation: [Apache License, Version 2.0, canonical text](https://www.apache.org/licenses/LICENSE-2.0.txt). The root NOTICE applies project attribution separately; the standard license appendix remains unchanged.
