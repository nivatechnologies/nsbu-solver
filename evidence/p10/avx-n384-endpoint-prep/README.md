# N384 accelerated endpoint profile preparation

This directory prepares, but does not freeze or run, the separately identified N384 endpoint
experiment. The preparation starts from integrated source `29d9f1c` and remains admission-closed
until the reviewed final W3 source is merged, the exact execution reservation is recomputed, and
the full feature-specific checks pass. `n384-prep` always returns `InvalidPayload` before state,
FFT catalog, force, RHS, observer, or artifact allocation.

## Pending immutable profile

The intended from-rest profile retains N384, samples the integration force on M384, uses
Cox--Matthews h32 with at most 128 accepted attempts, keeps the existing absolute and relative
tolerances, and applies the actual-stage advective guard with limit 0.8. The integration backend is
the finite-size RustFFT 6.4.1 AVX/AVX2/FMA catalog. The final source will own two separate opt-in W3
FFT pools: three persistent workers for the padded N384 RHS layout 576, and three persistent workers
for the M384 reduced-force transforms. The 32 reduced-force sampling workers remain a separate
owned pool.

The final W3 seam is expected to supply
`SpectralRhs::reservation_with_w3_fft_backend`, `SpectralRhs::new_with_catalog_w3`,
`SpectralRhs::w3_fft_identity`, and the corresponding
`ParallelReducedV2ForceW3::{preflight_with_fft_backend,new_with_catalog,identity,w3_fft_identity}`
interfaces. The provisional reviewed storage additions are 9,200,779,136 bytes for the RHS W3 pool
and 1,827,942,144 bytes for the provider W3 pool, totaling 11,028,721,280 bytes. These figures are
dependency inputs, not the final execution preflight. The execution byte cap remains unset until
the final W3 commit is merged and the complete catalog/provider/RHS/attempt/observer/stack/queue
reservation is checked together.

The same-host one-step probe wraps the final RHS in a harness-owned delegating timer. It measures
each of the 12 `evaluate` calls with `std::time::Instant`, includes timer call overhead without
subtraction, and records the sum, timed call count, whole `try_advance` wall time, and their
difference. The difference attributes coefficient construction, indicators, wrapper accumulation,
and other attempt work without caching coefficients or changing numerical inputs or outputs. The
timer adds no allocation and its storage overhead is included in harness overhead.

The observer runs at exact clocks 0, 512, 1024, 1536, 2048, 2560, 3072, 3584, and 4096. Rest is
the exact analytic record. Each of the eight positive nodes evaluates the force at M768 and the
conservative balance at 2N=768. No balance is fabricated at an unscheduled step, and completion of
the nested Simpson schedule makes no spatial or quadrature sufficiency claim.

## Every-step artifact contract

Rest publishes metadata only: no state payload is written. Every accepted positive proposal stages
one full nonresumable N384 snapshot, its attempt record, and its observation record before the
infallible in-memory commit. An unscheduled record says `observation_status=NotScheduled` and has no
balance field. A scheduled bundle says `observation_status=Scheduled` and includes the actual
balance and component timing. The scheduled record, attempt, and state use the same bundle rename.

After the in-memory commit, a successful directory rename followed by successful parent-directory
sync advances the durable attempt and state clock together. A rename failure can leave the bundle
at the `.partial` path. If rename succeeds but parent sync fails, the bundle can instead be present
at its final path while remaining unconfirmed. The failure status retains distinct attempted,
in-memory, confirmed-durable, and provisional frontiers and reports both candidate paths and their
observed existence. This protocol does not promise crash atomicity across memory and filesystem.

The artifact cap is exactly 274,877,906,944 bytes (256 GiB). The conservative 128-step bound is
174,862,106,624 bytes: each step reserves the 1,366,032,384-byte coefficient payload plus 77,824
bytes for header, records, and filesystem overhead. Snapshots encode finite coefficient words in
little-endian order and hash coefficient bytes with SHA-256. Bulk snapshots stay outside Git.

Execution is intended for exclusive whole-host Sulaco placement with an externally owned deadline
and process-group stop whose script, hash, PID, start time, deadline, source hash, binary hash, NUMA
policy, and exact command are captured before launch. No launch is authorized by this preparation.

## Prepared verification

Focused tests cover the exact schedule, admission refusal while W3 is pending, the 256 GiB artifact
bound, metadata-only rest, absence of a synthetic unscheduled balance, actual scheduled balance,
same-bundle durable frontier movement, precommit failure semantics, and both post-rename locations.
The preparation has 17 passing N384 tests plus 12 passing tests in each default and N256 regression
profile. Formatting and strict N384 Clippy pass. Maximum function cyclomatic/cognitive complexity is
15/7, function/file Halstead difficulty is 26.833/57.324, and the maximum file is 499 lines. Combined
N384/default coverage gives maximum per-function CRAP 22.5 with no violation. Focused N384 coverage
is 703/1,686 lines (41.70%) and 33/92 branches (35.87%); it is recorded honestly and is not the whole
maintained-scope 80% gate.

The source/profile identity must be refrozen only after the final W3 merge, exact preflight, strict
formatting and Clippy, file-size and complexity gates, and feature-specific coverage/CRAP checks.
