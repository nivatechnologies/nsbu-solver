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
