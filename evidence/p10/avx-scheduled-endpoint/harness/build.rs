//! Emits one named cfg so the offline-capture profiles share the transactional
//! capture path without duplicating the mutually exclusive scalar/parallel admission.
//! `n512-m512-parallel-capture` is enabled by every N512 parallel capture profile
//! (the reviewed 48-step piecewise profile and the two finer temporal refinement
//! profiles), so one detection covers all three.
fn main() {
    println!("cargo::rustc-check-cfg=cfg(capture_offline)");
    let capture = std::env::var("CARGO_FEATURE_N512_M512_PARALLEL_CAPTURE").is_ok()
        || std::env::var("CARGO_FEATURE_N256_M512_PIECEWISE_CADV33").is_ok();
    if capture {
        println!("cargo:rustc-cfg=capture_offline");
    }
}
