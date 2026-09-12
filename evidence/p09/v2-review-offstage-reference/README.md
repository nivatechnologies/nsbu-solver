# Off-stage reconstructed-reference adapter evidence

Source `ddd886474e716d27b472ea8dd4639ea78bb8aff2` validates and extracts the actual finest-CM reconstructed-reference sampled RMS at all seven manifest clocks. It remains a `PartialUnqualifiedInventory`; physical space/time/method values remain reconstructed physical pairwise errors and all other channels remain missing.

The selected LLVM adapter test passed with 28 records and zero steady allocations. Production adapter CRAP is 24.05859375. Raw `53ce` coverage/CRAP is retained: it failed CRAP53.125 before the reference validator was split. The corrected report passes. LLVM JSON did not uniquely map this integration-test source for a test-file CRAP report; the focused test itself passed and its raw metric file is retained.
