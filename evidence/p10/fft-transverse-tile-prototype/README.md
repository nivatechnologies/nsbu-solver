# Tiled transverse FFT prototype

This isolated prototype tests an eight-lane tiled gather/scatter around the existing scalar transverse FFT lines. Every line is still transformed once, in the original row and increasing-`k` order, by the same RustFFT or owned plan with unchanged normalization. The tile changes memory traversal only; no threading or backend rewrite is included.

`FftWorkspace` gains one `8 * max_axis` `Complex64` vector. Both real reservation paths use checked arithmetic; their existing object-size term records the added 24-byte `Vec` header. The observed catalog-workspace increments are exactly 12,312 bytes at N96, 24,600 at N192, and 32,792 at N256, equal to tile elements plus the header. Exact caps pass and one-byte-under construction is refused.

The final alternating order was untiled A, tiled A, tiled B, untiled B. Forward and inverse raw words are bitwise identical across both variants and rounds at N96/N192/N256. Their half lengths 49/97/129 each exercise a one-lane partial tail, and the deterministic input includes a z-Nyquist term. Frozen untiled comparisons on anisotropic `[6,96,192]` and `[96,6,192]` layouts are also bitwise identical forward and inverse for both AVX and Owned backends; these cover unequal axis lengths, a maximum larger than each transverse axis in turn, and a one-lane half-spectrum tail. A dense deterministic `[96,6,6]` control is likewise bitwise identical for AVX and Owned and exercises a four-lane partial tile. Fifteen focused FFT tests, the serial allocation process, catalog one-byte-under control, clippy with warnings denied, and complexity checks pass. Helper extraction reduced `transverse_axis` cognitive complexity from 25 to 8 without changing the measured algorithm.

Final median tiled time relative to untiled was 0.943 forward / 0.926 inverse at N96, 1.096 / 0.970 at N192, and 0.727 / 0.730 at N256. Thus the page-aligned N256 case improved about 27% in both directions, while smaller cases were mixed. The N512 endpoint projection owned the heavy localhost slot during these small tests, so these are shared-host exploratory timings. The result justifies a representative N768 A/B after resource handoff; it does not justify production adoption or a full-trajectory speedup claim.

`comparison.json` preserves both the simple and helper-extracted measurements, exact reservations and artifact hashes. Raw word files remain local and ignored because they are large; their hashes are bound in the comparison. The baseline binary SHA is `0c1d8835eb5e5c85fbf33176b3359e07c9884eb1280dc88d9590575a23b3a695`; the final prototype binary SHA is `68b74b72fed6ec937f8e5db04d5b54cd5bfdd62b298083510942039faf785119`.

The original sparse anisotropic receipts record binary hashes `254a320f...` and
`a7ca5fe9...`, but those binaries were subsequently overwritten before a source/build
receipt was frozen. Their word equality remains observed evidence, not a retrospectively
proven immutable build relation. The later dense width-four controls and prepared N768
binaries have explicit source, harness and build bindings. This historical limitation does
not block the prepared N768 comparison.

## N768 scalar A/B

After the endpoint projection owner explicitly released the host, the reviewed launcher
ran the frozen order untiled A, tiled A, tiled B, untiled B. All four rounds exited zero,
remained below the 24 GiB address-space limit, used no swap, and produced identical
forward and inverse canonical-word hashes. The workspace reservation increased by exactly
98,328 bytes.

The two-round medians were 13.9769017365 s untiled versus 10.883707544 s tiled forward,
and 14.1547264215 s untiled versus 11.2317807865 s tiled inverse. The combined median
improved 21.39%; forward improved 22.13% and inverse improved 20.65%. Per-round timings
varied substantially, so `raw-n768/comparison.json` retains every observation and the
alternating order. This passes the predeclared 10% exploratory selection heuristic and
supports preparing cross-source full-word W3 controls. It is not production adoption, a
PDE gate, or evidence of full-trajectory speedup.
