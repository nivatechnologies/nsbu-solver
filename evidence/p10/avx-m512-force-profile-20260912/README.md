# Prepared N384/M512 force-refinement profile

This isolated source prepares an experimental N384 trajectory whose integration
force grid is refined from M384 to M512. It does not authorize or launch a
trajectory. Architecture review is required before integration, deployment, or
launch.

The exact source is `000e80099a546f69076379387f8b2283602132c1`, based on combined
commit `33b44f5d6979b5c5e31a9ba967a58afb6c1d06e7`. The opt-in Cargo feature is
`n384-m512-piecewise-cadv33` in
`evidence/p10/avx-scheduled-endpoint/harness/Cargo.toml`. Existing features and
defaults retain M384. W3 admission adds only cubic layout 512 to the prior finite
set; layout 288 and adjacent valid layouts 510 and 514 remain refused, as does
the OwnedRadix backend.

The retained state and RHS geometry remain N384 and padded layout 576. The new
force owner alone uses layout 512. Observer force and conservative layouts both
remain 768. The schedule remains 32 steps of 64 ticks on `[0,2048)` followed by
16 steps of 128 ticks on `[2048,4096)`, with Cox--Matthews, advective limit 3.3,
absolute tolerances `[1e-5,1e-4]`, relative tolerances `[1e-5,1e-5]`, the same
nine observer nodes, and a full nonresumable snapshot after every accepted
step.

Preflight reproduces the reviewed reservation exactly:

| Quantity | Bytes/work |
|---|---:|
| Execution reservation | 193,243,071,240 B |
| Execution cap | 206,158,430,208 B |
| Cap margin | 12,915,358,968 B |
| RHS and cached force | 46,267,356,216 B |
| M512 forward W3 addition | 4,318,334,720 B |
| Attempt workspace | 12,749,636,728 B |
| Observer | 98,075,173,144 B |
| Integration work bound | 10,022,091,230,016 |
| Observer work bound | 467,480,346,624 |
| Artifact bound | 65,573,289,984 B |
| Artifact cap | 137,438,953,472 B |

The source includes an ignored, explicit environment-gated M512 force-only
control. It constructs serial and W3 providers sequentially, compares every
coefficient bit word and the complete `ForceWork`, validates the W3 identity,
and requires three repeated W3 evaluations to allocate, deallocate, and
reallocate zero times.

The first admitted execution failed the unchanged steady-allocation assertion
with `(2 allocations, 2 deallocations, 1 reallocation)`. Its stdout also shows
libtest's `has been running for over 60 seconds` report inside the measured
region. A single source-changed diagnostic at
`29adfa60287edea6e1d748ffcf07bc6c7fb5e1d0` reproduced the exact counts. All
five events came from pthread `129064448048128`, while the control owner was
pthread `129064444622528`. Symbolized frames resolve through
`test::formatters::pretty::PrettyFormatter::write_plain`,
`test::console::on_test_event`, and `test::run_tests`. The 512-byte allocation
and 82-byte string growth to 164 bytes form the monitor report's buffer
lifecycle. The W3 provider owner made no recorded allocation. Raw output and
symbolization are preserved under `review-diagnostic-20260912T2228Z`.

Root architecture review approved one final standalone closure. Commit
`1417be336a426b61e10a0112befb5bf71bc42594` adds an explicitly resource-gated,
harness-free executable owner. It keeps the original `stats_alloc::Region`
boundary around the same three first-attempt W3 evaluations and applies no
warm-up or owner-thread filter. Its only admitted execution passed exact serial
bit words, complete `ForceWork`, W3 identity, and global `(0,0,0)` allocation
counts with exit status zero. It completed in 124.32 seconds with peak RSS
15,278,380 KiB under a 32 GiB virtual-memory limit and 600-second timeout. The
closure record is preserved under `review-standalone-20260912T2237Z`; the
original libtest failure and attribution trace remain preserved separately.

The original invocation was:

```text
NSBU_RUN_M512_W3_FORCE_CONTROL=1 cargo test -p nsbu-benchmarks \
  --test parallel_reduced_w3_m512 -- --ignored --nocapture
```

The successful standalone invocation was:

```text
ulimit -v 33554432
NSBU_RUN_M512_W3_FORCE_CONTROL=1 /usr/bin/time -v timeout 600s \
  target/debug/deps/parallel_reduced_w3_m512_standalone-f4ef88baa3d93ce0
```

The inherited profile identity names Sulaco and its whole-host unbound NUMA
policy. Architecture review must confirm that placement before any deployment
artifact or launch recipe is created. No endpoint timing, trajectory,
force-sufficiency, convergence, or PDE-window claim is made; accepted PDE
windows remain zero.
