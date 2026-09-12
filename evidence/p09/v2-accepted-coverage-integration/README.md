# Accepted nominal-coverage diagnostic integration

Final source is `e595d9b1851a4af1de5884dd0ddc62eed98caff4`. It attaches independently evaluated core/annulus nominal geometry to the three accepted diagnostic events. It does not infer collar coverage, populate off-stage coverage, or qualify a concentrating PDE window.

Exact-source normal checks passed: region coverage 3/3 (0.15 s), diagnostic coordinator 3/3 (114.50 s), diagnostic export 3/3 (105.30 s), and the allocation executable. The allocator printed exactly `v2 diagnostic admission=0 construction_bytes=64022624 joint_bytes=74750488 events=7 execution_allocations=0`.

Exact-source static gates passed: format, workspace all-target Clippy under the CI warning policy, strict workspace Rustdoc, and repository validation. Whole Rust static maxima are cyclomatic 21, cognitive 21, all-node Halstead difficulty 75.9295154185022, and maximum tracked Rust file length 496 lines.

A clean exact-source instrumented run passed export 3/3 (616.23 s) and region coverage 3/3 (0.57 s). With the CI filename policy its four final-modified maintained files have maximum CRAP 21. The earlier isolated source-`3a6306748bf0a1af7e6376684e48180d6603b71c` profiles cover 13 byte-identical maintained files with maximum CRAP 20.671296296296294. `coverage/mod.rs` has no function, while `v2_cached_integration.rs` and `v2_review_adapter.rs` were compiled by all-target Clippy but were not executed in the focused profile; their coverage-dependent CRAP and the repository-wide 80% line/branch gate remain for hosted whole-workspace CI. This evidence therefore closes the changed production paths exercised here, not the global coverage gate.

The zero-panel review correction rejects `[0,2,4]` before divisibility and also rejects odd `[3,6,12]`. The final four changed test functions each have cyclomatic complexity 1, giving conservative zero-coverage CRAP 2.

The separately archived hardware-pause checkpoint preserves the interrupted and invalid-overlap history; those files are not counted as passing evidence here.
