# Clock 1024 matched-prefix comparison

The bounded adapter completed successfully on the reviewed N192 and N384
manifests. Both states use the same Cox--Matthews, M384, tolerance, and h64
evolution prefix through clock 1024. Execution backend and observer metadata
remain distinct and are reported in the raw output.

- Full-band difference: L2 `5.1829412722417726e-5`, H1
  `3.920723878398577e-2`, and vorticity L2 `3.9207204533639545e-2`.
- Relative to the N384 state: L2 `6.083978723559144e-5`, H1
  `1.6763808171490894e-3`, and vorticity L2
  `1.6774925305442102e-3`.
- The newly resolved shell contains `0.9999998987573991` of the squared H1
  difference. The common-band H1 difference is `1.2475211137129395e-5`.
- Absolute divergence difference is `2.1572533993141e-15`.
- The full H1 difference is `4.190952042872723` times the illustrative
  allocated spatial budget `4e-4 * ||u_N384||_H1` at this clock.

This is an actual trajectory spatial diagnostic and an end-to-end decoder
control. It is not an accepted-window result: it is only the coarse-to-fine
pair at one interior clock, and the remaining acceptance channels are open.
The shell split shows that a common-band-only comparison would miss nearly all
of this difference.

The 1.5 GB of staged snapshots are deliberately excluded from Git. Their file
and coefficient hashes, plan hashes, source identities, and transfer review are
recorded in the reviewed manifests and raw output.
