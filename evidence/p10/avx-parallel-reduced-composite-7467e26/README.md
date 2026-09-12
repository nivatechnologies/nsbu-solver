# AVX plus parallel-reduced direct trajectory profile

This isolated harness composes the opt-in RustFFT 6.4.1 AVX/FMA scalar backend with
`ParallelReducedV2Force` and the production Cox--Matthews attempt kernel. It does not enter
`RunForce`, `v2_run::Run`, a checkpoint path, or the balance observer. The measured contract is
therefore one direct 12-RHS attempt, not a complete accepted-step runtime under the current
cached/observed `Run` policy.

The binary embeds source identity `codex/p10-fft-batch-20260912@de6dd09`. The separately
reproducible finite-plan audit at `../avx-fft-storage-audit-7467e26.md` supports the fixed catalog
accounting reserve on this pinned host/profile. Accelerated trajectory arithmetic remains
unqualified and accelerated archives remain unsupported and refused.

## Results

| retained / force / workers | state entering attempt | seconds |
|---|---|---:|
| N192 / M384 / W32 | exact rest | 185.316809965 |
| N192 / M384 / W32 | startup-ramp state after one h16 step | 199.603770066 |

The retained 1126.717 s original N192/M384 measurement used attempt-cached integration and a
doubled-grid observer. It is not a matching denominator for this direct/no-observer harness, so
no composite speedup is reported from that comparison. Independent force-only and FFT-only gains
also cannot be multiplied into a trajectory claim.

Each measured attempt made 12 provider requests and charged 156 scalar transforms: 36 provider
transforms at M384 and 120 rotational-operator transforms at padded layout 288. Both attempts
reported zero allocations, deallocations and reallocations inside `try_advance`. State hashes were
`0e3eeebe7e732e7f3779d4460b88419408accfbdbc99dd25a2433931d5a644c7` after the rest attempt and
`8a5c151efb0775c5278b27d5caf348b2c83593e2433841803b2ec25872f116f4` after the nonzero attempt.

The pair used 392.91 s wall, 6139.28 s user, 5.00 s system, 1563% average CPU and 7,555,072 KiB
maximum RSS. Preflight reserved 12,018,944,768 bytes against the fixed 34,359,738,368-byte cap:
29,362,480 catalog bytes, 5,859,032,408 RHS/provider/operator bytes, 1,601,963,128 attempt bytes,
64 KiB explicit overhead, plus state/candidate storage charged by `ResourcePlan`.

A separate N64/M192/W32 control completed at 16.043805136 s from rest and 20.216652791 s from its
first nonzero state, also with zero steady allocations. It is a functionality check rather than a
large-grid performance gate.

One original trajectory job was concurrently using about 23 logical CPUs on this shared
128-logical-CPU host. The composite pair averaged about 15.6 CPUs despite W32 sampling workers,
so these wall times include shared-host and memory-bandwidth effects. They are not isolated
capacity measurements and do not establish the requested matched step-reduction gate.

At the measured startup-ramp cost, 256 h16 attempts to the first legal endpoint 4096 project to
51,098.6 s (14.19 h) before observers, retries and I/O. Clock 32 is not representative of the
later concentrating trajectory, so this is only a scale warning rather than an endpoint estimate.
The next bounded experiment is the actual five-clock attempt-cache protocol plus the current
doubled-grid observer at startup; scheduled observation follows only after that combined
bottleneck is measured. No cached or observed benefit is inferred by scaling the direct result.

## Reproduction

The commands are in `commands.txt`; the compiler source, lockfile, stdout, `/usr/bin/time -v`
records, executable hash and evidence hashes are listed in `MANIFEST.sha256`. The release binary
was built in the dedicated `target/avx-reduced-composite` directory.
