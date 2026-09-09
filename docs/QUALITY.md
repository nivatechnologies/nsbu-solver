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
