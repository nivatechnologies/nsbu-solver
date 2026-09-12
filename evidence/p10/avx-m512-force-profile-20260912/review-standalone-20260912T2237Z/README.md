# M512 standalone allocation closure

Root architecture review approved one final execution under a 32 GiB
virtual-memory limit and 600-second timeout. The source commit is
`1417be336a426b61e10a0112befb5bf71bc42594`; the executable source is
`crates/nsbu-benchmarks/tests/parallel_reduced_w3_m512_standalone.rs`.

The harness-free owner retains the original `stats_alloc::Region` immediately
before the first W3 evaluation and reads it immediately after the third. It has
no warm-up, owner-thread filter, or libtest monitor. Serial and W3 providers are
constructed sequentially. The executable requires exact coefficient bit words,
the complete `ForceWork`, W3 resource identity, and global allocation counts
`(0,0,0)`.

The sole admitted run passed every assertion and exited zero. Elapsed wall time
was 124.32 seconds; peak RSS was 15,278,380 KiB; no swap or major page faults
were reported. Binary SHA-256 was
`ce4385a2ec7624144329d5972db9b0776f9e7b58323af1d78bb0939b42cd3539`.
The original failed libtest execution and the source-changed attribution run
remain preserved in their sibling review directories. No further repeat is
authorized absent a new failure or source change.
