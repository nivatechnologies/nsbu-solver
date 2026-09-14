//! Emits one named cfg so the two offline-capture profiles share the transactional
//! capture path without duplicating the mutually exclusive scalar/parallel admission.
fn main() {
    println!("cargo::rustc-check-cfg=cfg(capture_offline)");
    let capture = std::env::var("CARGO_FEATURE_N512_M512_PIECEWISE_CADV33").is_ok()
        || std::env::var("CARGO_FEATURE_N256_M512_PIECEWISE_CADV33").is_ok();
    if capture {
        println!("cargo:rustc-cfg=capture_offline");
    }
}
