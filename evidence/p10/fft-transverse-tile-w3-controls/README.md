# Tiled FFT W3 cross-source controls

This isolated validation tree applies the reviewed eight-lane transverse FFT tile to the
completed layout-768 W3 control source at `ca42569`. Production source commit `335f674`
also repairs the W3 reservation tests for the added workspace. Nothing here changes the
frozen scalar A/B binaries or adopts the prototype.

The same private harness is built once against the untiled `ca42569` source and once
against the tiled source. Each build detects only one of two exact reviewed reservation
pairs. Force and RHS controls compare serial and W3 output bits internally, retain the
existing zero steady-allocation measurements, and now emit a canonical little-endian
SHA-256 over every complex output coefficient. The two source builds must produce the
same force hash and the same RHS hash. This supplies the required old-untiled versus
tiled full-word control rather than relying only on tiled serial-versus-W3 equality.

For layout 768, one scalar workspace grows by `8 * 768 * 16 + 24 = 98,328` bytes.
The W3 additional reservation contains two such workspaces and grows by 196,656 bytes:
forward 14,539,902,720 to 14,540,099,376 and bidirectional 21,787,660,160 to
21,787,856,816. At layout 512 the corresponding W3 increase is
`2 * (8 * 512 * 16 + 24) = 131,120`, giving forward 4,318,465,840 bytes.

The current tiled preflight reports whole-control peaks of 50,117,087,056 bytes for force
and 72,386,085,136 bytes for RHS. These are 32 and 40 bytes above the earlier desk ledger
because the actual APIs also reflect small type/header changes outside the two W3
workspace delta. Launch limits must bind the actual API values; this discrepancy requires
review before any large control is run. The existing 64 GiB and 96 GiB caps still contain
the measured preflight values.

Terra reported that 6 of 7 W3 tests passed before the stale reservation constants were
corrected, but no raw log was saved. That report is retained as a review note rather than
claimed as archived evidence. The local source-bound validation logs in `checks/` are the
first archived runs for the repaired constants.

No large force or RHS control has been launched from this tree.
