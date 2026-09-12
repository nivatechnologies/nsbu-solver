# Fixed-M96 N48 to N64 midpoint comparison

Two independent from-rest exact-v2 runs reached clock 2048 with 128 attempts and 128 commits. Both used frozen source `def4730b08025fdd06e7a8a0d78116aea24b6e2c`, M96 original forcing, 32 workers, and Cox–Matthews step 16. N48 ended successfully at `2026-09-12T08:49:08Z`; N64 ended successfully at `2026-09-12T09:14:50Z`. The compressed coefficient snapshots bind bit-for-bit to the declared raw hashes.

The complete-band N48 to N64 difference is L2 `0.028232131916258566` and H1 `5.716213563272909`. On the common N48 band, H1 is `0.02475909155452701`; the newly resolved shell contributes H1 `5.716159942507781`. Against the independently evolved finer N64 state's L2/H1 norms `1.741785215511511` / `47.6442377500622`, the difference ratios are `0.01620873323808046` / `0.11997701785596195`.

This extends the clock-2048 fixed-M96 spatial diagnostic. The finer-normalized H1 ratio falls from the prior N32 to N48 value `0.2752570355883181` to `0.11997701785596195`, while newly resolved modes still dominate the difference. A single decreasing adjacent ratio does not establish convergence. The separately sampled maintained M384 analytical-reference shell N48 to N64 had H1 `5.749099`; its proximity helps explain the measured shell contribution but supplies neither a trajectory-error bound nor a continuum-tail bound.

The comparator checked file lengths and supplied SHA-256 identities before reading finite f64 coefficient words. Its declared reservation was 18,509,824 bytes for the pair comparison and 25,956,352 bytes for the finer-state norm, each under a 256 MiB cap. Measured process maximum RSS was 19,456 KiB and 26,624 KiB respectively; RSS is not an allocation reservation.

`raw/n48/run-preflight.stdout` and `raw/n64/run-preflight.stdout` are the authoritative admissions for the executed runs. The run directories also preserve preliminary `preflight`, `deadline-preflight`, and `snapshot-checked-preflight` artifacts; some N64 preliminary files describe the earlier N48 draft and did not admit the N64 execution. They remain unchanged for provenance and must not be substituted for `run-preflight.stdout`.

These results compare fixed-force spatial states only. They establish no spatial convergence, force sufficiency, clock-4096 behavior, mandatory P10 channel completeness, concentrating window, or PDE qualification. Accepted windows remain zero.
