# Clock-1112 fine residual localization proposal

This proposal narrows the completed offline residual experiment to the fine Hermite support
`[1088,1152,1216]` at clock 1112. It records a design for review; it does not implement or run
the localization. No other clock, runtime owner, accepted interpolation, or accepted window is
in scope.

The existing signs are consistent. The archived-equivalent ETD rotational RHS omits linear
viscosity, and the node consumer adds `-nu |k|^2 u` once to form the full physical derivative
`D`. The conservative kernel returns `C768=P(div(v tensor v)-f768)`. The residual kernel then
forms

```text
V    = +nu |k|^2 v
R768 = D + C768 + V.
```

The localized path must preserve the completed kernel's operation order: initialize from C768,
add D, and then add each of the three viscous wave contributions separately. It may accumulate V
alongside those additions, but must not replace them with one pre-summed value. The replay gate is
the completed fine residual SHA-256
`0f156b5c1ca4470a34c0a1524601a7bc12e53ad9e86781c33cb48fe07d7bd9b8`.

For every non-N768-Nyquist mode, a successful `source.layout().locate(mode)` defines the strict
N384 retained band. Failure defines the N768 new shell. This is the same strict transfer rule that
omits source Nyquist ambiguity. The partition contains 28,164,288 retained modes and 197,738,688
new-shell modes; 1,179,264 N768 Nyquist storage slots remain zero and have zero weight.

Each region reports L2, H1, curl L2, and divergence L2 norms of D, V, C768, and R768 with the exact
`NormSums` Parseval weights and formulas. Full-grid squared sums are the sum of retained and shell
squared sums. Scaled compensated signed bilinear accumulators report the three pairwise cross
contributions and normalized alignments. For every channel, the cancellation factor is
`||R768||/(||D||+||V||+||C768||)`, or null when the denominator is zero. On the shell, D and V must
be exact zero and R768 must equal C768 bit-for-bit.

The additional force-resolution control samples a fresh M384 force into an N384 field with the
exact integration constructor: AVX catalog, width-three forward W3 cached parallel-reduced
provider, and 32 workers. Its attempt-cache epoch is synthetic and read-only, beginning at clock
1112 for 64 ticks so that its single evaluation occurs at the first allowed stage clock 1112.
The path strictly zero-pads that N384 field; it never asks an M384 provider to populate N768.
Linearity gives

```text
deltaF = P(f768 - pad(f384))
C384   = C768 + deltaF
R384   = R768 + deltaF.
```

The plus sign is required because C768 already contains `-P f768`. The control reuses the one
N768 conservative product. After that product consumes f768, its field can be overwritten
modewise by deltaF. C384 and R384 receive the same regional norms, cross terms, and cancellation
metrics without another N768 field. This control changes the discrete target equation outside the
retained band, so it is labeled as a force-resolution diagnostic rather than a production
residual.

Only the immutable `state.bin` files at clocks 1088, 1152, and 1216 are coefficient payloads.
The completed evidence serialized hashes of node derivatives, reconstructions, and residuals,
but no coefficient payloads for those objects. A rerun must hash-decode the three state files,
verify their frozen file and coefficient hashes, and freshly recompute all physical derivatives.
Their derivative hashes and the completed fine reconstruction hashes are frozen as replay gates
in `localization-proposal.json`. No prior derivative may be injected.

The checked reconstruction peak is 49,781,517,672 bytes. The conservative residual/control peak
is 114,575,696,272 bytes: three N384 fields cover reconstructed value, reconstructed derivative,
and retained M384 force; the bound admits both force-provider classes, the conservative workspace,
the catalog, ten existing N768 component fields, and fixed overhead. A scoped 128 GiB cap leaves
22,863,257,200 bytes above these owned classes. Scalar accumulators and hashers fit within fixed
overhead. A launch still requires fresh live `MemAvailable` admission after other reservations.

The exact formal provider bound is 87,994,073,108 work units. The run performs 54 scalar transforms:
39 for three node RHS evaluations, three for the M384 control force, three for the M768 base force,
and nine for the conservative product. Outside FFT internals, the conservative product has
5,755,355,136 explicit loop visits. Validation plus the single localized assembly traversal has
1,845,706,752 ComparisonPlan-style coefficient visits, and replay hashing has 681,246,720 visits.
These are coefficient-visit charges, not primitive-FLOP claims. Based on the completed 30:21
seven-node, three-residual run, the fine-only control is forecast at 18--24 minutes centrally and
35 minutes conservatively, retaining a 2700-second timeout and 60-second kill grace.

Before any heavy execution, focused controls must verify the selected M384 W3 constructor against
the archived scalar AVX path, the viscosity and force-delta signs, strict padding and shell zeros,
regional norm/cross reconstruction, and malformed-input refusals. All resulting quantities remain
empirical binary64 values without outward rounding or interval enclosure. They cannot establish a
velocity budget, residual pass, qualification, accepted interpolation, or accepted window.
