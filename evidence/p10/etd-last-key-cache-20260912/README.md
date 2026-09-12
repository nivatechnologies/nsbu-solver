# ETD last-key cache prototype

Base source: `ba2ad3904769dad0f8611daed307d352d8ec5d6a` from the N384 preparation lineage. This isolated prototype does not change active binaries, defaults outside its branch, checkpoint formats, or any N384 trajectory.

`AttemptWorkspace` now retains the exact `dt.to_bits()` and `half_dt.to_bits()` identity of its `MethodWorkspace` coefficient tables. The enclosing workspace fixes the `ResourcePlan` domain and method, so the 16-byte key cannot cross a domain or method owner. A hit returns before modal traversal. A miss marks the key invalid before the first in-place table write and publishes the requested bits only after both tables are complete. A downstream RHS or kernel failure leaves a previously completed table key valid because coefficients do not depend on state or RHS output.

The default `AttemptWorkspace` type grows from 784 to 800 bytes. `reservation_with_method` includes `size_of::<AttemptWorkspace>()`, so every CM and HO diagnostics reservation grows by exactly 16 bytes without a heap allocation or additional table. At the N384 CM profile, diagnostics reservation is 12,749,636,744 bytes. The N4 default reservation is 22,664 bytes; a 22,663-byte diagnostics class is refused before construction.

Coefficient tables and the key remain derived scratch. They are not written to physical or history checkpoints. A restored state receives a newly constructed cold workspace. The nonzero N4 force control proves that a same-step hit and a checkpoint-restored cold rebuild produce identical candidate words for CM and HO.

## N384 ZeroRHS result

The standalone harness used two owners with the same N384 rest state and target `ticks=8`. The forced-miss owner was first warmed at `ticks=4`; the cache-hit owner was first warmed at `ticks=8`. The measured requests both used `ticks=8`. The bounded RHS fills zero and records its own 12-call wall time, allowing `total - RHS` comparison.

| Case | Total | RHS | Outside RHS |
|---|---:|---:|---:|
| Forced miss | 16.804300711 s | 1.068942986 s | 15.735357725 s |
| Exact hit | 13.083795970 s | 1.051052333 s | 12.032743637 s |

The hit saved 3.702614088 seconds, or 23.5305% of forced-miss outside-RHS time. It passes the 20% fraction gate and fails the required 10-second absolute gate. Both candidates have SHA-256 `65db2aea053eb94f3d5b9c5c2dd8bd4be8dae247064a40cc13316441afa54460`, zero indicator bits, 12 RHS calls, and zero steady allocations.

The run used one fixed physical CPU, `nice -n 19`, a 24 GiB virtual-memory limit, and a 1,800-second timeout. Two unrelated unbound endpoint processes remained active, so absolute timings are contention-qualified. The result is negative under the approved gate; no combined or actual-trajectory measurement follows.

## Controls

- Complete CM and HO table words match across exact hits, different-step forced misses, and fresh rebuilds.
- A large finite step triggers a partial rebuild failure, leaves the key invalid, and a later valid request repairs every word.
- A downstream RHS failure retains the completed key and tables.
- Nonzero forced candidate words match a checkpoint-cold reconstruction for CM and HO.
- Default resource ABI, exact cap refusal, zero steady allocations, exact target candidate hashes, indicators, and RHS counts are checked.

Focused quality checks pass: strict Clippy, formatting, six cache/checkpoint controls, the dedicated attempt allocation binary, maximum changed production function CC 19, cognitive complexity 5, Halstead difficulty 48.243, and targeted maximum CRAP 19.
