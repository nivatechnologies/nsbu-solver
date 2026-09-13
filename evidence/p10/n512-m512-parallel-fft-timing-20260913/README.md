# N512/M512 shared-pool parallel FFT timing harness

Status: implementation and bounded preflight complete; no large attempt launched.

Source commit `477c418d30d9f2c8b117ae05240fc5596ecbd33b` integrates the compile-ready
parallel executor and the slab batching checkpoint. The explicit W3 path gives each force and RHS
operator one pool with eight helper workers and three persistent W3 callers. Each caller enters the
pool once for its entire worker lifetime, so numerical transforms use nested in-pool work rather
than recurring foreign Injector submissions. Existing scalar/W3 constructors remain unchanged.

The maintained allocator guard runs three callers through four warmups and 96 forward/inverse
rounds, compares exact scalar words, and observes zero allocations, deallocations, or reallocations.
A separate N=4/M=6 smoke constructs the actual cached reduced force and SpectralRhs through both
legacy and parallel W3 factories, compares exact output words, and checks the parallel cap-minus-one
refusal.

The parallel N512/M512 preflight is allocation-only and does not start a numerical attempt. It binds
the same rest-to-64 Cox--Matthews case, 32 sampling workers, tolerances, and five-node cache profile.
Its exact reservation is 207,626,205,968 bytes, 49,365,280 bytes above the legacy 207,576,840,688-byte
profile. The release binary and complete changed-source inventory are recorded under `prepared/`.
This remains diagnostic evidence and does not qualify or publish a PDE state.
