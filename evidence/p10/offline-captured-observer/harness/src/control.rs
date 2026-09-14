//! Test-only control: compile the reviewed w3-n256 harness observer verbatim so the
//! offline observer can be compared against the exact existing observer implementation.
#![allow(dead_code)]

#[path = "../../../avx-w3-n256-integration-20260912/harness/src/observer.rs"]
mod captured;

pub(crate) use captured::ReducedObserver;
