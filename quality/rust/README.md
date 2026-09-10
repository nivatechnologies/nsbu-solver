# Rust quality tools

The public crates need only Rust 1.94.0; these separately installed tools are for
verification. They are not linked into or distributed with the solver.

| Tool | Exact version | License | Purpose |
|---|---|---|---|
| cargo-llvm-cov | 0.9.1 | Apache-2.0 OR MIT | LLVM executable line and branch coverage |
| cargo-mutants | 27.1.0 | MIT | Mutation generation and test execution |
| rust-code-analysis-cli | 0.0.25 | MPL-2.0 | Cyclomatic, cognitive and Halstead metrics |
| jscpd | 5.2.0 | MIT | Token duplication detection |

Install the Cargo tools with `cargo install NAME --version VERSION --locked`.
Install jscpd using `npm ci --prefix quality/rust --ignore-scripts`; its complete
resolved package integrity inventory is in `package-lock.json`. Node 18 or later
is needed for this optional tool. Cargo tool dependency resolution uses each
published tool's lockfile; source is not vendored here.

The coverage-only toolchain is `nightly-2026-03-03`, with `llvm-tools-preview`.
Production builds stay on stable Rust 1.94.0. Exact commands and thresholds are in
[the workflow](../../.github/workflows/rust.yml). Coverage explicitly includes
integration-test source, overriding the tool's default exclusion. LLVM's unstable
branch instrumentation measures instrumented conditions, not MC/DC or every
semantic failure path. Review match arms and public API usage separately.

Mutation testing targets implementation code; test code remains in coverage,
complexity, size and duplication scope. Mutation and clone reports are
informational; unviable mutations are reported separately. Clippy denies warnings
except `dead_code`, while manual review checks exported API usage and intentional
independent numerical implementations. Rust's compiler resolves inferred types;
`std::any::Any` escapes are prohibited.

CI retains a broad lexical inventory of `Any`/`unknown` matches. Its refusal scan
skips full line and documentation comments, so ordinary English prose does not
masquerade as a type escape. All other matches require review; the scan remains
conservative for strings, inline comments and block comments. Rust compilation
provides type resolution independently. This lexical check is not a Rust parser.

Push and pull-request checks do not run a full mutation sweep. Start the optional
complete report through `workflow_dispatch` with `full_mutation_report=true`.
That run requires a complete, finished outcome file; survivor and timeout counts
are retained for review rather than used as a quality gate.
