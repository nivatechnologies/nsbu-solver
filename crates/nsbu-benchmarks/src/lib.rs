//! Independent prescribed-force and scalar-reference implementations for exact similarity-mms-v2.
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
