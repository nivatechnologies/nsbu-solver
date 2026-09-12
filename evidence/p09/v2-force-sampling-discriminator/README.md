# V2 force-sampling discriminator

This isolated run compares the unchanged prescribed-force providers sampled at
M24 and M48, both with 12 workers, after retaining their coefficients on the
same N24 grid. It evaluates the original force at exact clocks
1920/1984/2016/2032/2047/2048 from source commit
`31e99a17f97aec2ee18b26c67f8be88a0e931088`.

For each clock, the harness reports full-band raw M24--M48, raw M24 and raw
M48 norms, then applies the exact N24 Leray projector to each field and reports
the same three norm sets. The raw output also records a SHA-256 digest of every
coefficient payload. The projected comparison is relevant to the velocity
forcing contribution; raw curl is also projection-invariant.

The complete declared storage is 61,242,704 bytes:
26,925,608 (M24 provider) + 33,234,728 (M48 provider) + 1,078,272 (nine
retained N24 component arrays) + 4,096 explicit harness allowance. The run
made 12 provider evaluations, charged at most 96,297,984 provider work units
and 36 provider scalar transforms, and completed in 2.60 seconds under a
900-second timeout. It also declares 1,886,976 comparison visits and 269,568
projection coefficient visits.

At clock 2047, the projected force difference is L2 1854.1746972397223,
H1 140921.43129427923 and curl 140909.23260815843. This confirms that the M24
and M48 force discretizations differ materially after projection. It does not
compare a full RHS, use an evolved state, assess nonlinear aliasing, establish
temporal reconstruction error, or qualify a PDE window.

Reproduce from the repository root with:

```sh
/usr/bin/time -v timeout 900s cargo run --release \
  --manifest-path evidence/p09/v2-force-sampling-discriminator/harness/Cargo.toml
```
