# Local 27B capture disk-preflight tests

Qwen3.8 27B through OpenCode added three test-only checks for addition/multiplication overflow, exact disk-cap admission, and two-step amplification beyond the cap. Root reviewed the diff and independently reran the N512-feature step_artifact tests: 8 passed, no failures. No production code or live process changed. The tests use arithmetic only, not N512 state allocation.

Base source: `065c091be522205c08d1c1f54551e3c02b88cbaa`. Run command used that exact `RUN_SOURCE`, `cargo test --offline --manifest-path evidence/p10/avx-scheduled-endpoint/harness/Cargo.toml --features n512-m512-piecewise-cadv33 step_artifact::tests -- --test-threads=1`. These checks do not qualify a PDE window.
