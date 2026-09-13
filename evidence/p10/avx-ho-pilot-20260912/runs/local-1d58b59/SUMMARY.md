# Local contended N384 HO timing pilot

The sole numerical launch began 2026-09-12T23:46:24Z and exited successfully at 23:59:54Z under the exact true-v2 watchdog. A preceding launcher invocation failed while creating a missing parent directory and started no solver. Admission saw `MemAvailable=381351952384` B against the frozen floor 274877906944 B. Protected old and matched N256 solvers were live at launch; the old N256 completed naturally during the pilot, while the matched N256 remained live throughout.

The source-bound pilot constructed in 88.767370409 s. Its accepted REST HO h64 attempt took 383.681601824 s: 340.388826382 s inside 15 RHS evaluations and 43.292775442 s outside RHS evaluation. Accounting was exactly 10 cache hits and 5 physical force evaluations. Error ratios were 1.587793823152948e-8 and 4.324683895799838e-8. The attempt timing was durably published before observation.

The separate observer took 298.061442171 s: force 108.894467157 s, conservative 144.198211616 s, and transfer/measurement 44.968762007 s. The balance was discarded. The accepted token was dropped without calling `prepare_commit` or `commit`; `complete.json` records committed clock/epoch/accepted steps 0/0/0, no state payload, no balance, and no frontier. Both attempt and observer allocation regions enforce zero allocations, deallocations, and reallocations; exit status 0 proves both checks passed.

GNU time records 787.35 s wall, 3848.15 s user, 96.02 s system, 161542144 KiB maximum RSS, zero swaps, and exit 0. A live `/proc` poll observed VmHWM 161547236 KiB; this poll was not a continuous high-water monitor. stderr is empty. The run is a contended local measurement and does not establish quiescent speed or later-state/h128 cost.

Using the measured integration for all 48 piecewise attempts, the measured observer for the eight positive nodes, 48 snapshots at the prior measured 4.75 s each, and one construction gives 21117.975795329 s raw. Adding 10% yields 23229.773374862 s (6 h 27 min 9.8 s), requiring launch by 00:32:50Z to finish by 07:00Z. This is a conservative planning bound from one REST timing, not a trajectory result.
