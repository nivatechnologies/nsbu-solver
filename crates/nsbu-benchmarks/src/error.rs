//! Typed mathematical and finite-arithmetic refusals for the fixed v2 benchmark.

/// A benchmark request cannot be evaluated under the declared binary64 profile.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BenchmarkError {
    /// Input is nonfinite or outside the supported mathematical domain.
    InvalidInput,
    /// The exact clock does not identify the v2 target time.
    ClockIdentity,
    /// Finite binary64 arithmetic cannot resolve the requested operation.
    ArithmeticResolution,
    /// Safeguarded scalar iteration reached its declared finite work cap.
    RootWorkExhausted,
    /// Requested diagnostic quadrature exceeds its explicit evaluation allowance or supported size.
    DiagnosticWorkExceeded,
}
