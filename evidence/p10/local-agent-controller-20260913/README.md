# Bounded local controller checks

The focused controller suite passed 18 tests. Strict basedpyright and repository
checks passed. Maximum controller cyclomatic complexity was 13, cognitive
complexity 19, and Halstead difficulty 9.75; the largest file had 498 lines.
All recorded commands exited zero. The basedpyright log preserves a missing
worktree-local virtual-environment notice; checks used the prepared parent venv.

Controller changes were committed as 375008c. Root subsequently normalized
indentation at 9b816f1 and asserted Python AST equivalence. The source-sha receipt
was written after that normalization. These are focused checks, distinct from
the full-suite coverage run and any hosted CI or PDE acceptance gate.

The native endpoint smoke used earlier revision dfb6ff1; its receipt is in
`../local-modal-derivative-fixtures-20260913/native-tool-smoke.json`. Later phase
refusals and time-budget guards have deterministic regression tests, not a
repeated full endpoint campaign. No scientific result is automatically accepted.

## Broader integration run

The Python suite passed 257 tests in 562.33 seconds under a 600-second guard.
Its interim report records 6,767/7,068 executable lines and 1,324/1,420 branches
covered. This run began before the final controller test and parsing changes;
it is preserved as interim integration evidence, not a final-source coverage
gate. The separate pre-boundary-tests report exposed the controller CLI/HTTP
coverage gap (411/548 lines and 116/176 branches), prompting targeted tests.

## Final controller result

Source `2cbad43` passes 32 focused tests, 497/548 executable lines (90.69%)
and 141/176 branches (80.11%) covered, and maximum CRAP 22.5. Strict typing,
complexity and repository checks pass. `final/coverage.json`, `final/radon-cc.json`
and `final/crap.json` supersede earlier reports. The final summary distinguishes
the last 32-test result from the retained earlier 26-test log. These checks cover
the controller package, not a source-matched whole-maintained release gate.
