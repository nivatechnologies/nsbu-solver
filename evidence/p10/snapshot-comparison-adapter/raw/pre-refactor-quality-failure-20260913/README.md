# Pre-refactor quality failure

This directory preserves the fresh reports produced after the initial mechanical
`mixed.rs` module split. The run had 21 passing tests and strict Clippy passed,
but `check_crap.py` exited nonzero. Large-only mixed orchestration/output paths
were not exercised by the small fixtures; current unrefreshed Hessian/profile
decode paths also exceeded the per-function CRAP gate. These reports are retained
as the input to the bounded responsibility and focused-test repair. They are not
a passing quality result.
