# Hardware-pause checkpoint: accepted nominal coverage integration

Source commit: `3a6306748bf0a1af7e6376684e48180d6603b71c`.

The fresh isolated LLVM command was interrupted at the explicit hardware pause. Its raw log records completed `regions` (7 passed), `v2_diagnostic_coordinator` (3 passed, 755.50 s), and `v2_diagnostic_coordinator_allocation`, whose exact output was:

```
v2 diagnostic admission=0 construction_bytes=64022624 joint_bytes=74750488 events=7 execution_allocations=0
```

The process had started `v2_diagnostic_export`; that test did not complete. `v2_region_coverage`, the requested final JSON generation, and CRAP analysis therefore did not run in this isolated command. This is partial evidence, not a coverage or CRAP pass.

The two `coverage-invalid-*` logs came from accidentally overlapping commands that shared an output profile. They are retained only as operator-error history and provide no evidence.

The stopped command used the unique target directory `target/llvm-cov-final`; it exited 130 after SIGINT. No owned process remained after shutdown.
