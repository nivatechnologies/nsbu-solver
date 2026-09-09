//! NSBU Solver public facade. Numerical packages follow the reviewed plan.

/// Public project name.
pub const NAME: &str = "NSBU Solver";
/// Development package version; it does not identify a validated solver release.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod domain;
mod error;
pub use error::SolverError;

/// Binary64 complex Fourier coefficient from the pinned public arithmetic dependency.
pub type Complex64 = num_complex::Complex<f64>;
