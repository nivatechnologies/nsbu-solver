# N256/M512 fixed-M512 coarse-branch capture preparation

This packet implements an opt-in `n256-m512-piecewise-cadv33` capture profile for
the existing scheduled-endpoint harness and does not deploy, launch, SSH, or run
any attempt. It prepares — it does not execute — the coarse end of the fixed-M512
spatial family beside the completed N384/M512 r6 anchor and the prepared
N512/M512 profiles; the N256/M512 coarse trajectory itself remains unexecuted. It
changes no numerical kernel, arithmetic, tick clock, bounded-attempt,
transactional publication, resource/disk/work-guard, or from-rest behaviour of any
existing feature; the reviewed N512 and every N384/N256/N192 profile still compile
and pass their focused tests unchanged.

## Profile identity (retained 256, force 512, RHS dealias 384)

Case `similarity-mms-v2` on the unit cube with `nu = 1`. The profile retains 256,
samples force at 512, and pads quadratic products to the 384 dealiased layout
(`256 * 3 / 2 = 384`). It starts from exact rest at clock exponent `-20` with
target 8192, runs 32 steps of 64 through clock 2048 and 16 steps of 128 through
4096, uses Cox--Matthews, absolute tolerances `[1e-5, 1e-4]`, relative tolerances
`[1e-5, 1e-5]`, advective limit 3.3, and 48 bounded transactional attempts. The
8 positive clocks 512, 1024, 1536, 2048, 2560, 3072, 3584, 4096 are marked as the
offline observer schedule.

The profile reuses the reviewed scalar/component W3 ownership (`w3` forward force
and `w3` bidirectional operator, three persistent callers, no parallel FFT pool)
and the reviewed `f13c29c9…` W3 source. Root will run N256 on Baccus concurrently
with the Sulaco N512 capture, so this identity is the only one that reports
`host=baccus`; every existing N512/N384/default identity string keeps
`host=sulaco` unchanged. It also carries `external_stop=pgid-watchdog-v3-confirmed-identity-absolute-deadline`
because root will drive it with the existing reviewed generic
`proposed-launch/pgid-watchdog-v3.sh`. The identity is explicit N256 and never
mislabels as N384 or N512: `profile=n256-m512-h64to2048-h128to4096-cadv33-w3-f13c29c`,
`retained=256`, `force_samples=512`, `rhs_dealias=384`,
`rhs_w3=layout384-width3-bidirectional-add2734010240`,
`force_w3=layout512-width3-forward-add4318465792`,
`schema=p10-avx-n256-m512-observer-state-v1`, `host=baccus`, and the v3 stop. A
focused test asserts the identity contains `host=baccus` and the v3 stop and
contains none of `host=sulaco`, `pgid-watchdog-v2`, `n384`, `n512`, or `parallel8`.

## Capture-every-actual-state, offline Baccus

The profile shares the reviewed offline-capture path with N512 (a build-script
`capture_offline` cfg): every accepted proposal is staged before commit and
published as a create-new, synced state bundle after commit, with
`observation_status: CapturedActualState` and `observer_execution:
offline-baccus-required`. No inline expensive observer runs and no substitute or
zero balance is emitted: the harness reserves zero observer bytes, records no
observables, sets `qualification: false`, and provides no resume decoder. The 48
actual committed states are the input for later off-stage and reconstruction
diagnostics on Baccus under a separately reviewed envelope. The N256 dealias and
M512 scalar W3 identity constants are validated in-repo by the owners
`exact_w3_identity_values_accept_and_any_change_refuses` test (no large array is
allocated; only reservation arithmetic runs).

## Exact API / disk / work preflight (PRELIGHT only)

The exact conservative API reservation is `37,402,593,776` bytes: catalog
`29,362,480`, force storage `15,015,554,504`, RHS `22,866,181,176`, attempt
`3,787,457,656`, observer `0`, capture overhead `1,310,736`. One byte under the
total is refused before allocation. The declared execution cap is `68,719,476,736`
bytes (64 GiB), leaving `31,316,882,960` bytes of headroom; this is a coarse-branch
ceiling, not the host limit, so a gross regression is still refused. The exact
state-bundle disk bound is `19,482,083,328` bytes, well below the 128 GiB artifact
cap (`137,438,953,472`). The integration work bound is `9,987,522,825,024` units and
the offline observer work bound is `138,512,695,296` units. Full capture is in
`preflight/n256-m512-preflight.stdout`; the corrected host/stop candidate
supersedes the earlier sulaco/v2 candidate, whose raw output is retained
byte-for-byte under `preflight/historical-candidate-rev1/` rather than overwritten.

## Focused commands (all status 0)

```
H=evidence/p10/avx-scheduled-endpoint/harness/Cargo.toml
RUN_SOURCE=$(git rev-parse HEAD) cargo test   --manifest-path $H --features n256-m512-piecewise-cadv33
RUN_SOURCE=$(git rev-parse HEAD) cargo clippy --manifest-path $H --features n256-m512-piecewise-cadv33 --all-targets -- -D warnings
RUN_SOURCE=$(git rev-parse HEAD) cargo build  --release --manifest-path $H --features n256-m512-piecewise-cadv33
RUN_SOURCE=$(git rev-parse HEAD) cargo test   --manifest-path $H --features n512-m512-piecewise-cadv33   # unchanged, 22 pass
RUN_SOURCE=$(git rev-parse HEAD) cargo test   --manifest-path $H --features n384-m512-piecewise-cadv33   # unchanged, 22 pass
RUN_SOURCE=$(git rev-parse HEAD) cargo test   --manifest-path $H                                          # default n192, 12 pass
```

The compiled fixture was built at `RUN_SOURCE=2ca1a1ce07a7127a34b937ba062bc45704e2557b`
(the current `HEAD`), but the working tree carries uncommitted changes; treat the
binary and preflight as an uncommitted candidate until the root review freezes
them. Feature-conflict compile refusals are recorded in
`smalltests/mutual-exclusion.stdout`.

## Source changed paths (exclusive ownership: this profile only)

Within `evidence/p10/avx-scheduled-endpoint/harness/`: `Cargo.toml` (one added
feature), a new `build.rs` (`capture_offline` cfg), and `src/config.rs`,
`src/main.rs`, `src/owners.rs`, `src/step_artifact.rs`, `src/artifact.rs`
(`cfg`-growth that shares the reviewed scalar-W3 admission and the reviewed
offline-capture path; no new FFT path, no numerical-kernel change). `config.rs`
and `main.rs` remain above 500 lines — a preexisting, previously-documented
exception for the run-critical transaction/schedule structure that this additive
change preserves rather than split.

## First-attempt readiness and unknowns

Ready: the profile compiles clean under `-D warnings`, PRELIGHT admits with the
exact reservation and refuses one byte under, the run gate is opt-in via
`NSBU_RUN_N256_M512_ENDPOINT_CAPTURE=1` (default refuses), the mutual-exclusion
guards refuse every conflicting profile, and existing profiles are unaffected.

Unknown before a real first attempt: whether an N256/M512 attempt is *accepted* by
the local CM tolerances at advective limit 3.3 on the concentrating window;
per-step integration, state-write and archive timing; and the actual archive
throughput. Root accepts the `2N=512` offline observer (force samples 512 and
conservative diagnostic domain `512`, consistent with
`ConservativeWorkspace::diagnostic_domain(2N)`) for this coarse diagnostic only —
it is not enlarged to the 768/1024 grids of the N384/N512 siblings, and it carries
no acceptance claim. No large attempt,
deploy, commit, push, SSH, signal, or production-trajectory change was made, and
no launcher that starts automatically was written. Passing PRELIGHT and these
focused tests does not qualify a PDE window.
