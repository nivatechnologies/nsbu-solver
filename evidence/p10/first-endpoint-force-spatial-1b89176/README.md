# First-endpoint fixed-force spatial screen

The independently evolved N48, N64, and N96 branches reached the first legal
endpoint, clock 4096 of target 8192 at exponent -20. They use the original
mathematical v2 force sampled on M192, Cox--Matthews h64, 32 force workers,
unchanged local tolerances and advective limit 0.3, and exact source
`1b89176f63737bbeb3ed65c7e3c7fe6dbeef8f02`.

The N48-to-N64 H1 difference is `5.75044324853682`, or
`11.334749426576345%` of the N64 state norm. The N64-to-N96 H1 difference is
`3.585227185610707`, or `7.049307343885922%` of the N96 state norm. These are
113.35 and 70.49 times the total pilot H1 target, respectively, and 283.37 and
176.23 times the allocated spatial share. Newly resolved modes account for
more than 99.997% of each squared H1 difference. The observed decrease is not
close to the frozen spatial budget and does not establish convergence.

The matched N128 h64 branch refused on the unchanged advective guard at clock
1728, after 27 committed steps. Its terminal state and failure remain retained.
A separately frozen guard/timestep study is evidence for another execution
profile; it cannot turn this refusal into a completed N128 member.

The first launch used a process deadline that measured startup throughput showed
could truncate a valid run. Those three status-143 launches remain under
`runs/`; the process-bound amendment precedes the completed relaunches. The N48
recovery plan also precedes its launch. Raw stdout, GNU time records, statuses,
timestamps, compressed snapshots, hashes, and hash-bound comparator outputs are
retained.

This is an unqualified fixed-force feasibility screen. M192 is not force
qualified, the three grids are underresolved, and mandatory time, force,
arithmetic, reference, physical, pressure, balance, residual, reconstruction,
perturbation, and lineage channels remain incomplete. Accepted windows remain
zero.
