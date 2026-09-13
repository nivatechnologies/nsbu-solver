# N512/M768 one-attempt timing probe

This standalone harness prepares one actual Cox--Matthews attempt from exact rest at N512 with
M768 cached forcing and a padded-768 W3 rotational RHS. It calls the typed `try_advance` API once
for 64 ticks, records timing and the local comparison result, and drops any local acceptance token
without committing or publishing a state. It has no observer, trajectory loop, archive, resume,
state import, reference assignment, or PDE acceptance path.

The exact clock is `TickClock::from_rest(-20, 8192)`. The sole interval is clocks 0--64, physical
duration `64 * 2^-20 = 1/16384`, with stage clocks `[0,16,32,48,64]`. Cox--Matthews makes 12 RHS
calls; the existing five-slot cache should report five misses and seven hits.

The API-derived closed peak is 238,209,735,152 bytes: 85,554,364,416 base `ResourcePlan` classes,
29,362,480 shared FFT catalog, 122,443,729,976 cached-force-inclusive W3 RHS,
30,182,212,728 attempt workspace, and 65,552 bytes of timing/output allowance. The M768 cached
force contributes 59,788,791,240 bytes inside the RHS figure and is not additive. All owners live
through the attempt. Preflight asserts the exact class vector and refusal at one byte below the
closed peak through the real APIs.

The force limit is 58,637,156,357 work units and three scalar transforms per cache miss; the run
also records the complete RHS consumption. Exact W3 identities require forward additional storage
14,539,902,720 bytes and bidirectional additional storage 21,787,660,160 bytes.

The command remains refused unless `NSBU_RUN_N512_M768_ONE_ATTEMPT=1` is set. No numerical run is
authorized by this source. The independent full M768 forward-force and padded-768 bidirectional-RHS
controls passed bitwise and allocation checks in the combined base lineage; source and binary
identities and an exclusive host resource plan still require separate review before launch. A local
accepted token, if produced, is reported only as an integrator outcome and is never a PDE or window
acceptance claim.

Any external launch receipt must enforce an actual 256-GiB address-space limit
(`274,877,906,944` bytes) and fresh `MemAvailable >= 292,057,776,128` bytes before constructing
the catalog or any numerical owner. The output's `published:false` means no trajectory state is
published; the JSON itself is a create-new diagnostic record.

Focused tests must run serially because the process-global `StatsAlloc` regions can otherwise
observe allocations from another test thread:

```text
RUN_SOURCE=<SOURCE_COMMIT> cargo test --manifest-path \
  evidence/p10/n512-m768-one-attempt-timing/harness/Cargo.toml -- --test-threads=1
```

Release compilation and the archived preflight must use the same frozen `RUN_SOURCE`; the binary,
source files, and preflight receipt are hash-bound before any launch review.

The frozen preflight source is `6442ec98bf55582e1990aac874e3b0234252add8`. Its release binary
SHA-256 is `0df5f007d26d836819f582a7028bb5534ca8a5c5d3047831c8022fac1f6f0e88`.
`prepared/implementation.json` records the exact launch bounds and zero executed numerical attempts;
`prepared/source-sha256.json` binds the harness and reused production sources.

`launch-one-attempt.sh` reuses the reviewed foreground-timeout process-group pattern. It refuses
while projection PID 1586745 exists, then requires the exact binary and source hashes, fresh
`MemAvailable`, create-new run storage, and a stable PID/PGID/starttime/cmdline identity before
recording a launch receipt. `prepared/launch-plan.json` binds the command and all limits. This does
not authorize launch before explicit projection-owner resource release and root handoff.
