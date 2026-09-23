//! Byte-exact frozen identity/backend/execution bindings for the completed
//! N384/M512 r6 endpoint lineage. Generated once from the reviewed input
//! manifest work/matched-m512-spatial-20260920/inputs/clock4096-left.json of
//! the p10-numerical-window-20260912 worktree; values are frozen constants.

pub(crate) const FINE_IDENTITY: &str = "source=326eeb5cbd5ebe39a7d5f7be77f9acfab8d0db72;case=e1236f7b3c51537acd17381402ca420ba7872a7b9dbc64b2f0d9d5108a468f7e;profile=n384-m512-h64to2048-h128to4096-cadv33-w3-f13c29c;backend=rustfft-6.4.1-avx-avx2-fma;w3_source=f13c29c9ae91d0b8cf7a790132deb9bd076911c0;provider=parallel-reduced-v2-force-w3-attempt-cache;rhs_w3=layout576-width3-bidirectional-add9200779136;force_w3=layout512-width3-forward-add4318334720;rhs_timer=harness-timed-rhs-v1;clock=std-time-Instant;scope=evaluate-inclusive;overhead=included;retained=384;force_samples=512;observer_force_samples=768;observer_conservative=768;sampling_workers=32;rhs_w3_workers=3;provider_w3_workers=3;method=cox-matthews;schedule=h64-clocks0-through2048-then-h128-through4096;endpoint=4096;advective_limit=3.3;execution_cap=206158430208;artifact_cap=137438953472;schema=p10-avx-n384-every-step-v1;attempt_schema=p10-avx-scheduled-attempt-v3;resume=unsupported;host=sulaco;numa=whole-host-unbound-all-visible-cpus-memory;external_stop=pgid-watchdog-v2-starttime-cmdline-deadline";

pub(crate) const FINE_BACKEND: &str = "rustfft-6.4.1-avx-avx2-fma";

pub(crate) const FINE_EXECUTION: &str = "provider=parallel-reduced-v2-force-w3-attempt-cache;rhs_w3=layout576-width3-bidirectional-add9200779136;force_w3=layout512-width3-forward-add4318334720;sampling_workers=32;rhs_w3_workers=3;provider_w3_workers=3;host=sulaco";
