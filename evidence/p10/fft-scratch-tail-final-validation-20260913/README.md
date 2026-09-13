# AVX scratch-tail final workspace validation

This archive preserves two distinct validations at source
`0843b8b18e6a096a0208e3d896e391c7b1b2f5e0`. It does not represent their union
as one successful `cargo test --workspace` invocation.

The original clean-target workspace command ran with `CARGO_BUILD_JOBS=2`,
`--locked`, `--no-fail-fast`, and `--test-threads=1`. Its fixed 5400-second
limit expired while `v2_regional_tracking` was starting, so `original/status.txt`
records 124. All 82 completed libtest result blocks before the interruption were
successful: 272 tests passed, zero failed, and one ignored test was not run. The
foreground timeout left test PID 2127386 in PGID 1956764. The recorded start
clock and command-line hash were revalidated before sending TERM to that PID;
it exited without KILL and the group was empty before any continuation began.
The automatic receipt hashes remain in `original/SHA256SUMS`; the later cleanup
and stable raw-file hashes are in `original/FINAL_SHA256SUMS`.

The separately frozen continuation derived the remaining inventory from Cargo
metadata and the original stderr `Running tests/...` records. It reran the
interrupted target and the 16 benchmark targets that had not started, without
repeating earlier completed targets. All 17 target executables completed with
phase status zero: the nine libtest-formatted blocks report 32 passed and zero
failed, while the standalone allocation targets completed under Cargo's zero
status. The subsequent `nsbu-cli` bins/tests report 26 passed and zero failed;
workspace doctests report one passed and zero failed. All three phases and the
overall launcher exited zero by 18:52:55Z, before the fixed 19:47:54Z cutoff.
No ignored heavy-test gate was enabled.

The solver package was not repeated: its separately owned corrected-source
validation had already reported 251/251 passed at `df90af8`. Comparing that
source with this archive's source shows identical benchmark code and identical
production code; the only code difference is three expected reservation
literals in `crates/nsbu-solver/src/spectral/w3/tests.rs`. See
`continuation/source-comparison.txt` and `continuation/result.json`.

No production file was changed during this validation. Both original and
continuation process groups were empty at final release. One existing dead-code
warning in the `v2_sampling_oracle` test was emitted; there were no test errors.
