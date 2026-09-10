//! Bounded Fourier Navier–Stokes integration with independent empirical diagnostics.
//!
//! Start with [`domain::Domain`], an exact [`domain::TickClock`] and a complete
//! [`domain::ResourcePlan`]. Allocate a committed [`domain::SpectralState`], a private
//! [`integrators::transaction::CandidateState`] and the selected method's
//! [`integrators::attempt::AttemptWorkspace`] before requesting numerical work.
//! Force providers declare finite costs; analytical references stay outside state updates.
//!
//! An attempt compares a full step against two half steps and returns a single-use
//! acceptance token. It leaves the committed field unchanged. The [`experiment`] layer
//! prepares independent measurements and the raw history before committing them together.
//! A local acceptance is an error-controller decision, not PDE convergence evidence.
//!
//! [`diagnostics`] contains full-band norms, conservative-product pressure and residuals,
//! balances and Hermite reconstruction. [`verification`] reviews supplied empirical
//! measurements; [`lineage`] and [`checkpoint`] retain identities and continuation data.
//! Imported bytes and lineage declarations do not establish physical provenance.
//!
//! APIs are experimental. The crate has no private-service dependency, no accepted
//! concentrating PDE window, and no claim of continuous-time error enclosures.
//! Build the workspace API manual with `cargo doc --workspace --no-deps`.

/// Public project name.
pub const NAME: &str = "NSBU Solver";
/// Development package version; it does not identify a validated solver release.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

pub mod domain;
mod error;
pub use error::SolverError;

/// Binary64 complex Fourier coefficient from the pinned public arithmetic dependency.
pub type Complex64 = num_complex::Complex<f64>;

pub mod spectral;

mod storage;

pub mod integrators;

pub mod diagnostics;

pub mod verification;

pub mod lineage;

pub mod experiment;

pub mod checkpoint;

#[cfg(test)]
mod test_support;

#[cfg(test)]
extern crate self as nsbu_solver;
