//! Independently evaluated diagnostics; measurements do not themselves qualify a PDE window.
pub mod balances;
pub mod comparison;
pub mod conservative;
pub mod derivatives;
pub mod hermite;
pub mod history;
pub mod integrity;
pub mod local;
mod modes;
pub mod norms;
pub mod quadrature;
pub mod residual;
pub mod sampling;
mod squares;
mod summation;
pub mod tails;

pub mod reconstruction_history;
