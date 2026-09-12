# Exact-v2 attempt-local force cache

`runtime_force::AttemptForceCache` is an opt-in provider wrapper for experiments that use the
original exact-v2 prescribed force. The default `v2_run::Run`, its settings, work ledger and its
version-1 archive remain unchanged and uncached.

An integration attempt evaluates its prescribed source twelve times with Cox--Matthews or fifteen
times with Hochbruck--Ostermann. The full step and two half steps use the same five exact clocks:
start, quarter, midpoint, three-quarter and endpoint. The wrapper owns five complete three-component
coefficient slots. At the start of every attempt it derives those clocks through
`TickClock::stages`, advances a private generation and invalidates every slot. The first call at a
clock evaluates the owned `RunForce`; later calls at that clock copy the published coefficients.
It never caches observer force calls or values from another attempt.

The provider publishes a slot only after the complete original-force evaluation succeeds. Calls
outside an admitted attempt, calls beyond the conservative fifteen-call allowance, clocks outside
the five-stage manifest, changed limits and wrong output layouts refuse. A rejected integration
attempt leaves no reusable cache state because the next attempt hook invalidates all five slots.

Preflight retains the underlying serial or persistent-worker provider reservation, fifteen complex
coefficient arrays, their allocator allowance and wrapper metadata. Per-call aggregate RHS work
includes the actual original-provider report on a miss, exact-clock comparisons and the copy of all
three retained coefficient fields. `AttemptCacheWork` reports those categories separately, including
provider evaluations/work/transforms, lookups/comparisons, copied coefficient words, hits and misses.
Provider failures retain their full admitted provider charge and never publish a slot.

This phase supplies the core hook and provider only. It makes no runtime speedup claim, changes no
force sampling or arithmetic, and has no cached-run checkpoint format. A future runtime owner must
bind this resource/work profile explicitly and either introduce a distinct archive version or refuse
cached archives. Version-1 archives continue only under the uncached profile.
