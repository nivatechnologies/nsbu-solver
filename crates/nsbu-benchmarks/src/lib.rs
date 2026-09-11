//! Independent manufactured-force/reference implementations and bounded smooth diagnostics.
//!
//! [`scalar`], [`jet`] and [`fields`] implement the frozen [`CASE_DEFINITION`]. The
//! separately bounded [`provider`] supplies the prescribed force to the integrator;
//! a reference field never replaces or resets an integrated state.
//!
//! [`smooth::CyclicSine`] is a different, smooth verification problem. Use
//! [`smooth_run::SmoothPlan`] to preflight it without numerical-grid allocation and
//! [`smooth_run::SmoothRun`] to evolve it from rest with independently measured balances.
//! Run `cargo run -p nsbu-benchmarks --example smooth_from_rest` for a bounded example.
//!
//! Smooth validation and small concentrating diagnostics do not qualify the frozen
//! concentrating problem. All reported samples retain their spatial and arithmetic limits.
mod error;
pub use error::BenchmarkError;
pub mod jet;
pub mod root;
pub mod scalar;
pub mod time;

pub mod fields;

pub mod provider;

/// Byte-preserved reviewed case definition. Historical status text describes the frozen review.
pub const CASE_DEFINITION: &str = include_str!("../data/similarity-mms-v2.json");
/// SHA-256 of the reviewed definition, checked against both copies by repository CI.
pub const CASE_SHA256: &str = "e1236f7b3c51537acd17381402ca420ba7872a7b9dbc64b2f0d9d5108a468f7e";

pub mod regions;
pub mod smooth;
pub mod smooth_observer;
pub mod smooth_run;

pub mod smooth_experiment;

/// Explicit serial/parallel force settings for the concentrating runtime.
pub mod runtime_force;

pub mod v2_experiment;
pub mod v2_run;
