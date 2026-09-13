# Direct local-model bounds pilot

This evidence evaluates a narrow, fixed-input code task against the locally served
`qwen3.8-flash-next` model. The LAN address is omitted. There was no cloud fallback,
repository exploration, tool loop, numerical run or maintained-source integration.
The functions operate on sampled binary64 norm inputs; they are not certified outward
rounding or PDE acceptance bounds.

The first thinking-disabled generation returned in 31.61 seconds but included Markdown
fences and failed compilation because a test called `abs` on an ambiguous float. Its
independent thinking-disabled review incorrectly returned `pass`, missing that compile
failure and two invalid overflow oracles. Both failures are retained.

A thinking-disabled repair returned in 32.19 seconds and passed all six embedded tests.
The first independently generated black-box suite failed to compile after inventing an
`InvalidInput` variant and repeating the ambiguous-float error. Its thinking-disabled
repair compiled and passed 22 of 23 tests, but used an incorrect non-unit-floor oracle:
`2 + 5/2` was asserted to be `7` rather than `4.5`. A final thinking-enabled repair
corrected only that oracle and passed all 23 tests: six embedded and 17 independent.
This demonstrates useful bounded
repair behavior after exact compiler/test feedback; it does not qualify broad autonomous
repository work.

`repair1.rs` is the delivered implementation. `independent-repair2.rs` is the final
independent suite. The earlier source, review, and test attempts remain alongside them.
