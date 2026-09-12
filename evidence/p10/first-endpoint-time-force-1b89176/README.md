# First-endpoint time, method, and force screen

All three branches start independently from rest and reach the first legal
endpoint, clock 4096 of target 8192 at exponent -20. They use retained N96,
the original mathematical v2 force, 32 workers, unchanged local tolerances,
and source `1b89176f63737bbeb3ed65c7e3c7fe6dbeef8f02`.

At M96, CM h64 versus h32 differs by `1.2256229678712585e-7` H1,
or `2.4095058355575032e-9` relative to the h32 state. CM versus HO at h64
differs by `1.3708326525277625e-7` H1. These small discrepancies do not
qualify the time or method channels for the refined-force problem: at fixed
N96 and CM h64, changing the force sampling grid from M96 to M192 changes H1
by `0.8282200257556894`, or `1.62845398845163%` relative to the M192 state.
That is 108.6 times the allocated H1 force budget.

The frozen plans precede each launch. Raw stdout, GNU time records, exit
statuses, start/end times, compressed snapshots, snapshot hashes, and
hash-bound comparator output are retained. Comparator metadata is externally
declared, so every comparison is bound back to a successful endpoint forensic
line and snapshot SHA-256.

This is a feasibility screen. It has only two CM timestep settings, and its
M96 time/method agreement is inseparable from the large force-grid error.
Mandatory arithmetic, reference, local, pressure, residual, reconstruction,
perturbation, and lineage evidence remains missing. Accepted windows remain
zero.
