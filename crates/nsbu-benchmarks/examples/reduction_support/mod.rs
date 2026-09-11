//! Exact-word diagnostic input and production statistics; not an execution or checkpoint importer.
pub mod input;
pub mod output;
pub mod reduce;
use nsbu_solver::SolverError;
use std::{fmt, io};
/// Failures retain their I/O or numerical origin; no partial result is successful.
#[derive(Debug)]
pub enum AuditError {
    Io(io::Error),
    Numerical(SolverError),
}
impl From<io::Error> for AuditError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
impl From<SolverError> for AuditError {
    fn from(error: SolverError) -> Self {
        Self::Numerical(error)
    }
}
impl fmt::Display for AuditError {
    fn fmt(&self, out: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Io(e) => write!(out, "{e}"),
            Self::Numerical(e) => write!(out, "{e:?}"),
        }
    }
}
impl std::error::Error for AuditError {}
pub const CAP: usize = 64 * 1024 * 1024;
