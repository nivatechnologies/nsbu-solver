# Quality evidence

The active implementation plan defines the required SOLID review and quantitative
gates. They have not yet been measured comprehensively. Passing the ten imported
bootstrap guard tests does not imply 100% coverage or zero surviving mutants.

The initial responsibility review identifies repository validation, mathematical
execution and CLI/report I/O as distinct responsibilities. The verification runner
imports repository validation rather than duplicating frozen identity logic. The
older provisional audit duplicates that logic and is superseded by the imported
checker; its failure report is retained as historical evidence only.

Next work: select and pin Python and Rust metric, strict typing, branch coverage,
mutation and duplication tools; inventory their language support and limitations;
review the imported maintained tools; establish measured baselines and close gaps.
No unsupported metric receives a passing value. The independent reference and
future production numerical kernels must remain independently implemented.

## Initial measured baseline

See [measurement evidence](../evidence/quality-baseline.json) for the full source
inventory, tool versions, commands, hashes and raw reports. Coverage includes
maintained source, tests and both reference studies; bootstrap child processes
are not instrumented. Coverage remains below 100%. Complexity, Halstead and file
size have initial passing measurements; the full quality gate does not pass.
Cognitive complexity, CRAP, mutation, dead-code, duplication and strict typing
remain unmeasured. No timeout, omission or unknown measurement is counted as zero.

The numerical responsibility review keeps scalar field formulas separate from jet
algebra, and coefficient convolution separate from direct DFT/grid products. CM
and HO coefficient construction are separate. Integration receives a mathematical
RHS; the fixture's prescribed force accepts only grid selection and time and never
reads integrated velocity. No analytical field is assigned into evolving state.

## Expanded reference measurements

The [current source-hashed report](../evidence/quality-reference/summary.json)
records 78 passing tests, 100% executable-line and branch coverage (1,948 lines,
410 branches), maximum cyclomatic complexity 12, cognitive complexity 15,
Halstead file difficulty 9.888 or less, 243 physical lines per file, and CRAP 12.
Strict basedpyright analysis reports no errors, including explicit and implicit
`Any`/unknown-type diagnostics. Pylint reports zero duplicated blocks. Seven
Vulture findings were individually reviewed as executed test classes or serialized
TypedDict schema keys; none is confirmed dead code. These supersede the initial
baseline for the exact source hashes in the report.

Mutation remains an unmet repository gate. The decorated jet implementation was
checked separately with Cosmic Ray: 601 generated, 592 killed, nine raw survivors
individually classified as equivalent in the recorded execution profile, zero
timeouts and zero abnormal results. The report preserves each surviving diff and
its justification. mutmut skips decorated classes, so its results alone cannot
establish the jet mutation gate. A comprehensive implementation mutation run is in
progress; neither this subset nor equivalent classifications imply repository-wide
zero survivors.

Reproduce the coverage and type checks with `coverage run -m pytest -q`,
`coverage combine`, `coverage json`, and `basedpyright --outputjson` after installing
`requirements-quality.txt`. Complexity uses `radon cc -j -s reference tools`,
`radon hal -j -f reference tools`, and `complexipy reference tools --output-format
json --output work/cognitive.json`. Static review uses `vulture reference tools`
and `pylint --disable=all --enable=duplicate-code --output-format=json reference tools`.
CRAP is calculated per function from Radon's complexity and coverage.py's branch
counts using the formula in the active plan; raw measurements are preserved next
to the summary. Mutation work runs in isolated copies because Cosmic Ray modifies
its target files in place. Never execute that mutation runner against the working
checkout while implementation work is underway.

SOLID review: tuple shape checks and JSON boundary narrowing have dedicated small
modules; field evaluation, quadrature, DFT operators, time steps and report drivers
retain separate responsibilities. Independent scalar differentiation and jet
algebra remain separate, as do explicit convolution and grid-product evaluation.
The trajectory integrators receive only an RHS callable, never a reference-state
assignment interface. Report-flow doubles test orchestration only; numerical
claims require separately executed high-precision studies.
