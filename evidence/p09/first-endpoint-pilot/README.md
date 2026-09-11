# Concentrating first-endpoint feasibility (diagnostic only)

This bounded study used the published `alpha-20260911` binary built from source
commit `6170341c42a64348c7d99bdfd4fc3653454f22a8`, not the subsequently advancing
main checkout. Every numerical branch started from exact rest. The immutable case,
fixed tolerances, viscosity, guard, and reference were unchanged. All executions
were sequential, individually limited to 120 seconds with a 256 MiB logical preflight cap, and totaled well
under 15 minutes. These results claim zero accepted PDE windows.

## Results

| N | M | step ticks | workers | reservation bytes | wall s | peak RSS KiB | endpoint/result | endpoint L2 | endpoint H1 | energy defect | enstrophy defect |
|---:|---:|---:|---:|---:|---:|---:|---|---:|---:|---:|---:|
| 8 | 16 | 128 | 0 | 2,979,480 | 5.79 | 4,096 | rejected on attempt 3 at tick 256 (t=1/4096) | 0.00153177 | 0.0252277 | -2.05181e-6 | -5.54764e-4 |
| 8 | 32 | 128 | 8 | 55,295,832 | 7.22 | 21,504 | rejected on attempt 3 at tick 256 (t=1/4096) | 0.00154949 | 0.0255249 | -2.06574e-6 | -5.590999e-4 |
| 8 | 16 | 64 | 8 | 38,544,728 | 34.69 | 5,120 | completed tick 4096 (t=1/256), 64/64 | 1.87864 | 35.4898 | 1.17273 | 561.249 |
| 16 | 16 | 64 | 8 | 47,535,832 | 69.01 | 12,288 | completed tick 4096 (t=1/256), 64/64 | 2.28199 | 62.8241 | 5.05809 | 9,058.82 |
| 8 | 32 | 64 | 32 | 159,142,232 | 89.40 | 22,528 | completed tick 4096 (t=1/256), 64/64 | 1.57016 | 27.9096 | -0.0646383 | -20.6921 |
| 16 | 32 | 64 | 32 | 168,133,336 | 112.83 | 28,672 | completed tick 4096 (t=1/256), 64/64 | 1.78384 | 43.0301 | 0.0316999 | 244.969 |

The 128-tick setting is unusable at N8 for both tested force grids under the frozen
local-error policy. Halving to 64 ticks reaches the endpoint without rejection.
Completion is only a terminal diagnostic result: changing M from 16 to 32 materially
changes the endpoint norms, while changing N from 8 to 16 increases H1 by about 77%
at M16 and 54% at M32. The large and nonmonotone balance defects reinforce that no
convergence or accepted-window inference is available.

For a 64-tick branch, the admitted/actual provider work is the dominant scaling:
M16 charges 3,191,444 integration-provider and 2,105,276 observer units; M32 charges
25,263,076 and 16,793,380, approximately eight times as much when M doubles. Increasing
N from 8 to 16 at fixed M and worker count adds substantial spectral/diagnostic time
(34.69 to 69.01 seconds at M16; 89.40 to 112.83 seconds at M32). M32 was tested with
32 workers; this study did not establish the minimum worker count needed to meet the
wall-time budget. The optional reduced provider has an earlier force-only median 6.48x speed
ratio, but it is outside released CLI selection and has distinct arithmetic identity.

## Recommended next study

The economical next scientific pilot is a dedicated admitted from-rest spatial family
at fixed M16, CM, 64 ticks, t=1/256, and the same finite worker count on N8 and N16.
It costs about 104 seconds for the two measured branches and directly targets the
currently increasing derivative-sensitive spatial channel. It must expose full-band
state differences, physical pressure, reconstructed/off-stage residuals, regions and
balance history through the validated family consumers; endpoint scalar norms from
independent CLI runs cannot substitute for those comparisons.

A subsequent force-sampling family should compare M16/M32 at fixed N and fixed worker
count. This feasibility run used 8 workers for M16 and 32 for M32, so its wall times
are not an isolated force-scaling comparison, even though tested provider paths have
separate exact-word equivalence evidence. Retain fixed workers and exact arithmetic
identity in the actual family. Do not adopt the reduced provider in that comparison
until its identity is explicitly bound to runtime plans and the study is designed for
the changed arithmetic.

Selected CLI report fields and timing measurements are in `runs.ndjson` and
`checkpoint-runs.ndjson`; exact commands are in `commands.txt`; resource preflights
are in `preflight.ndjson`; binary/source/case hashes are in `provenance.txt`.

The NDJSON records retain selected execution/report fields. Checkpoint binaries remain in ignored working storage; only their hashes and execution summaries are retained here. These are feasibility measurements, not a full experiment artifact or accepted-window record.
