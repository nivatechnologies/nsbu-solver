# Persistent parallel exact-v2 force sampling

`provider::parallel::ParallelV2Force` samples the same three-dimensional
`similarity-mms-v2` force as `V2Force`, using a fixed set of persistent workers.
It retains every spatial sample and the original pointwise arithmetic. After all
workers finish, the existing serial FFT and retained-band transfer produce the
output coefficients. The default `V2Force` and uncached point evaluator remain
available for comparison.

This optional backend is force evaluation infrastructure. It performs no PDE
integration and supplies no new force-accuracy, aliasing or trajectory claim.

## Execute a bounded comparison

```sh
cargo run --release -p nsbu-benchmarks --example parallel_force_profile -- --dry-run
cargo run --release -p nsbu-benchmarks --example parallel_force_profile
cargo run --release -p nsbu-benchmarks --example parallel_force_profile -- 32 --dry-run
cargo run --release -p nsbu-benchmarks --example parallel_force_profile -- 32
```

The default retained grid is N=16 with M=24 force samples per axis. Explicit
N=4/8/16/32/64 profiles are supported; M=3N/2. Every profile compares the serial
sampler with 1/2/4/8/16/32 workers, omitting worker counts larger than M. Exact
clock requests are 1/4/2/1 with quantum 2^-10 and target 8. The backward request
and repeated time test complete cache rebuilding.

The example preflights every alternative before allocating. Its 256 MiB joint
cap includes the serial comparison provider, one parallel provider, output
spectra and a 64 KiB formatting/metadata allowance. Each printed plan includes
configured worker storage and stack allowances. Timing covers force evaluation;
construction, formatting and hashing occur outside the measured interval.
Every parallel coefficient hash must match the corresponding serial hash or the
example returns a failing status. These hash comparisons are reproducibility
checks, not bounds on mathematical force error.

Larger profiles can take substantially longer. Use their explicit dry-run first;
small-grid timings and theoretical worker counts do not predict a PDE run time.

## Ownership and exact work

Worker w owns planes k=w+rW, where W is the configured worker count and r indexes
its local planes. Within each plane it visits every x/y sample. Its private
buffers use the original coordinates and scalar arithmetic. Each active z plane
solves the implicit root exactly once, so the total root iteration count and
reported point-assembly work match the serial cache. Uneven plane partitions and
anisotropic layouts are supported.

After collection, the owner copies each worker's disjoint plane words into the
original global physical-array order. No reduction combines samples from different
workers. All three FFTs and normalized retained transfers then use the original
serial implementation. Concurrent scheduling cannot reorder floating additions
inside those FFTs or change the local point formula.

The worker pool receives an exact benchmark time and owns force scratch only.
It has no analytical-reference assignment, integrated velocity, accepted-history
or checkpoint interface. Elapsed and remaining time conversions stay distinct,
as described in [force evaluation](FORCE_EVALUATION.md).

## Resource contract

`ParallelV2Force::preflight(domain, samples, workers)` is allocation-free and
requires 1 <= W <= min(M_z,128). Construction refuses an insufficient cap before
allocating an FFT plan, sample buffer or thread. All workers acknowledge startup
before the constructor returns. No worker is created, joined or resized during a
normal force evaluation.

The reservation includes:

- The original serial force/FFT owner, including its small axial scratch cache.
- Three additional physical arrays totaling 24 M_x M_y M_z bytes across workers.
- One axial-root entry per sample plane, partitioned across workers.
- Worker-vector, shared-state, mutex/condition and allocator metadata allowances.
- A configured 2 MiB stack and an additional 64 KiB native-thread/TLS allowance
  per worker, separate from the numerical heap buffers.

The retained serial axial scratch is deliberately included even though parallel
sampling uses the workers' plane caches. Reusing the established FFT owner costs
this small extra storage; it does not hide an unreserved allocation. Stack/native
allowances describe the tested execution profile. They are planning reservations,
not a measurement or universal bound on operating-system resident memory or its
thread-stack cache. Concurrent solver owners and other process memory need their
own budgets.

The maximum remains 129 times the number of sampled points, plus three separately
counted scalar transforms. Actual work counts one bounded point assembly/copy and
safeguarded scalar iterations once per active axial plane. These abstract units
are not primitive operations or wall-clock bounds. Worker scheduling and copies
have additional finite overhead even when the numerical work counts agree.

In the isolated M=6/8/12, three-worker check, construction made 37 Rust allocator
requests totaling 69,992 bytes. The configured stacks add 6,291,456 bytes; the
full reservation is 6,560,680 bytes. The first, repeated, nonmonotone and refused
force requests allocate, deallocate and reallocate nothing after construction.
These measurements apply to the recorded toolchain/platform, not every allocator.

## Failure and shutdown

The owner submits bounded jobs, then collects every submitted completion even
when one worker fails. No worker result is transformed into output coefficients
until the complete physical sampling pass succeeds. A worker error terminates
the pool; later requests are refused. Previously returned coefficients and the
last successful root-work report remain unchanged. Partial physical scratch is
private and cannot become an integrated state.

An unexpected programming panic is caught at the worker boundary and wakes the
controller with a terminal error. Poisoned sample data is never copied. This
emergency failure path is distinct from the normal allocation-free numerical
contract. Tests verify that even this completion is drained before returning.
Native scheduling waits do not supply a hard wall-clock deadline.

Dropping a worker signals shutdown and joins its thread. This also applies to
workers already constructed when a later construction fails. Idle and submitted
shutdown tests verify that all shared buffers are released after the join. The
integrator's existing candidate/commit transaction remains responsible for
rejecting an unsuccessful force/FFT attempt without advancing physical state.

## Current verification scope

```sh
cargo test -p nsbu-benchmarks --test parallel_force --test parallel_force_allocation
cargo test -p nsbu-benchmarks --lib provider::parallel
```

Tests compare every retained coefficient word and actual root-work count for
1/2/5/12-worker anisotropic partitions, including rest and nonmonotone requests.
Capacity, worker-count, domain, exact-clock, output-shape and declared-limit
refusals precede output changes. Separate controls fail each of three workers,
exercise an unexpected panic, verify all completions were collected, and check
idle/submitted shutdown. Full workspace quality and hosted checks remain required
before this increment is reported as verified.

P08/P09 and concentrating validation remain incomplete. No concentrating PDE
window is accepted by this backend or its timing profile.
