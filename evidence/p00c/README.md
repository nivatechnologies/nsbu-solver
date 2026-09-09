# P00C concentrating diagnostic

Both independent 80-digit Python trajectories start at exact rest and reach the
first reviewed endpoint, t=1/256, on N=4. Each takes 32 fixed steps of 1/8192.
CM performs 128 RHS evaluations and HO 160. Each samples the prescribed exact-v2
force at 65 unique dyadic times, totaling 4,160 field evaluations and 24,320 scalar
root iterations. The pure cache does not depend on the integrated velocity and
supports HO's nonmonotone stage requests. Reference fields are accessed only after
integration and never assigned to the evolving state.

| Result | CM | HO |
|---|---:|---:|
| Retained-band reference L2 error | 1.64125621035 | 1.64125621031 |
| Retained-band reference H1 error | 14.5754565929 | 14.5754565925 |
| Guard or candidate failure | none | none |
| Accepted PDE windows | 0 | 0 |

The CM–HO H1 difference is about 1.0395e-8, despite their shared large tracking
error. This demonstrates why agreement between methods sharing an unresolved
force/grid cannot qualify a trajectory. Existing N=4/8/12 force-sampling studies
already show unresolved spatial differences. No concentrating temporal/space
refinement family, trajectory arithmetic refinement or slab-wide enclosure is
claimed here. Both final spectra, all force-sampling reports and exact clocks
are preserved in the linked JSON reports.

Preflight reserves 202,706,944 bytes per branch, including 21,565,440 bytes for
the force cache, against a 1 GiB cap. This is a conservative allocating Python
diagnostic reservation, not measured RSS or a hard allocator guarantee. Fixed
step and stage counts are bounded; every advective guard is checked before new
force work. Failure tests verify that a rejected candidate does not advance the
clock or replace the prior spectrum, and that reference failures preserve the
integrated diagnostic record.

Sixteen added tests cover admission, exact schedules, complete initial rest,
post-integration reference access, force caching, work limits, signed sampling,
reporting and rollback. Mutation checks cover all 786 generated pilot mutations:
774 explicit failures and 12 individually justified equivalents; zero unresolved
survivors or timeouts. The complete Python scope is measured in the
[quality report](../quality-reference/summary.json). The pilot's earliest mutation
runs had timeouts; they remain archived and are resolved by explicit bounded-work
assertion failures in the final reconciliation.

SOLID review: force/RHS sampling owns no analytical reference interface. Resource
admission, evolution, tracking and report serialization have separate functions;
force caching and exact-stage validation are isolated from state replacement.
The reference is compared only after evolution. Python allocation and data-model
limits remain separate from the Rust transaction contract.

See [summary.json](summary.json) for exact source/artifact hashes and reproduction
commands. Numerical diagnostic checks pass; hosted CI is required before P00C is
marked complete. Rust numerical integration remains a later package.
