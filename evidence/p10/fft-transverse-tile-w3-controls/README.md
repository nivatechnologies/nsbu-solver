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

The matching untiled and tiled API preflights report whole-control peaks of
50,116,792,040 versus 50,117,087,056 bytes for force, and 72,385,790,112 versus
72,386,085,136 bytes for RHS. The observed increases decompose as three scalar workspace
increments plus 32 bytes for force and plus 40 bytes for RHS. These are 32 and 40 bytes
above the earlier desk ledger; this evidence records the API deltas without assigning the
small remainder to an unverified type or padding cause. Launch limits bind the actual API
values. The existing 64 GiB and 96 GiB caps contain them.

Terra reported that 6 of 7 W3 tests passed before the stale reservation constants were
corrected, but no raw log was saved. That report is retained as a review note rather than
claimed as archived evidence. The local source-bound validation logs in `checks/` are the
first archived runs for the repaired constants.

`launch-controls.sh` prepares the fixed order untiled force, tiled force, untiled RHS,
tiled RHS. It rechecks both source trees, the identical harness sources, binary hashes,
memory, disk, quiet-host state, and the campaign deadline before every round. Each worker
has an identity-bound process group, foreground timeout, 60-second kill grace, and the
reviewed 64/96 GiB address-space cap. The launcher refuses unless
`CONTROL_LAUNCH_AUTHORIZED=1` is explicitly supplied. At launcher freeze, no large force
or RHS control had been launched from this tree.

## Completed prototype controls

The authorized four-control sequence completed at `2026-09-13T16:57:41Z`. All rounds
exited zero with no swaps or major faults. Untiled and tiled force outputs have identical
SHA-256 `a5a74f81...a65b`; untiled and tiled RHS outputs have identical SHA-256
`7fbbe90c...5c73`. Every round independently reports serial/W3 bit equality and zero
steady allocations. Wall times were 12:08.88 / 11:56.98 for untiled/tiled force and
11:18.43 / 10:17.29 for untiled/tiled RHS. These are run conditions, not a controlled
performance comparison.

Subsequent architecture review found that this prototype's added `Vec` changes the public
owner layout covered by the v1 checkpoint compatibility contract. The successful controls
remain evidence about its arithmetic, traversal, and allocations, but the implementation
is excluded from production adoption. A replacement design must preserve the public owner
layout and repeat the tiled-side controls against the canonical untiled hashes.
