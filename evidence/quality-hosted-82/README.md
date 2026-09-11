# Hosted verification at source 82c0796

Both the [Rust run](https://github.com/nivatechnologies/nsbu-solver/actions/runs/34563652523)
and [Python/bootstrap run](https://github.com/nivatechnologies/nsbu-solver/actions/runs/34563652520)
passed on immutable commit `82c079670287b6db36737ec0f56cc3f3a69ee672`.
These results cover the six-trajectory exact-v2 family and optional pointwise
reduced evaluator before the physical consumer and sampled reduced provider.

The Rust instrumented suite passes 440 harness test cases plus 11 isolated
allocation executables (451 tests/probes). One public Rustdoc example passes
separately. Coverage measures 30,242/30,683 executable lines (98.5627%) and
2,095/2,358 branches (88.8465%). Maximum CRAP is 24.3359375; maximum cyclomatic
complexity 21, cognitive 18, function/file Halstead 75.8956 and physical file lines 477.
The declared maintained inventory contains 350 Rust files; LLVM emits 346 files
with executable coverage records. Source-only module aggregators remain in the
maintained inventory and static analysis. There are zero type-escape code findings.

Python passes 220 tests, 5,512/5,522 statements (99.6526%) and 1,085/1,098 branches
(98.8160%). Maximum CRAP is 16. Both jobs pass their remaining pinned quality,
repository and packaging checks. Informational Rust duplication records 1,510
lines (3.52096%); Python reports zero duplicate clones and 10 dead-code entries.
The yanked crossbeam-channel warning arises while installing rust-code-analysis,
not from the NSBU solver dependency lockfile.

The compressed raw archive retains every downloaded log/artifact and source
revision file. The archive inventory records their original sizes and hashes;
summary.json records source-matched run IDs and measurements. Quality results
are not force-grid or PDE convergence evidence. No PDE window is accepted.
