# Exact-v2 layout guard repair

Source base `335f674` added an eight-lane transverse FFT tile as a new `Vec` field on the shared
`FftWorkspace`. That changed the direct v2 owner layouts and honestly increased three resource
classes, so the pinned admission guard refused before allocation. The guard was retained.

A clean `ca425` build measured `RunForce/Plan/Run/ReconstructedPlan/ReconstructedRun` as
`592/512/5840/480/7136`. The first tiled form measured `608/512/5936/480/7232`; its tiny direct
resource classes changed from `[27648,15552,6912,2304,0,36376,152552,10840]` to
`[27648,15552,6912,2304,0,37792,154696,10936]`. This was real tile storage, so lowering caps or
updating only the layout literals would either understate storage or invalidate the historical
version-one direct archive contract.

The repair restores the shared workspace object and OwnedRadix implementation to their prior
layout. Only AVX workspaces extend their existing scratch allocation: four maximum-length lanes
remain the FFT scratch prefix and eight maximum-length lanes form the transverse tile tail. AVX
validation checks the RustFFT requirement against the prefix and the complete allocation length.
Every AVX transform receives only the prefix. OwnedRadix keeps its original scratch length,
reservation, traversal, owner layouts, resource classes, and historical checkpoint compatibility.
Same-layout workspaces from different backends now reject before transforming because their
scratch identities differ.

The repaired layouts are again `592/512/5840/480/7136`. All 15 FFT integration tests pass, as does
the exact historical direct layout/resource/version-one archive fixture. These checks change no
numerical tolerance or archive codec.

The initially reported clean-baseline `ResourceLimit` failures were caused by reuse of a tiled
Cargo target; a clean isolated `ca425` target passed the reproduced pressure test. Those failures
are not attributed to the baseline source.
