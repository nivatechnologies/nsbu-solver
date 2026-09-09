//! Structured refusals for invalid input and bounded resource exhaustion.

/// A solver operation failed before changing committed physical state.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolverError {
    /// Dimensions, lengths or viscosity do not define a supported domain.
    InvalidDomain,
    /// An index or integer mode is outside the represented grid.
    InvalidIndex,
    /// A checked element or byte count overflowed its addressable range.
    SizeOverflow,
    /// The exact clock state does not satisfy its defining invariant.
    InvalidClock,
    /// An interval is zero or does not provide integral quarter-stage ticks.
    InvalidStep,
    /// The finite integer clock cannot represent the requested experiment.
    ClockCapacityExceeded,
    /// An epoch counter cannot advance without wrapping.
    EpochExhausted,
    /// The complete declared storage exceeds the caller's cap.
    ResourceLimit,
    /// The allocator refused an otherwise approved reservation.
    AllocationFailed,
    /// A payload does not match its declared layout.
    InvalidPayload,
    /// Non-finite, Nyquist or conjugacy constraints failed.
    InvalidSpectrum,
    /// Required binary64 arithmetic cannot represent the exact requested quantity.
    ArithmeticResolutionLimited,
    /// Acceptance identity no longer matches the committed or candidate storage.
    StaleAttempt,
    /// External scheduler exhausted its explicitly finite local-error retry allowance.
    RetryLimit,
    /// A bounded profile cannot admit a provider with undeclared or invalid costs.
    UnknownProviderCost,
    /// Provider work, transforms or invocation count exceeded the declared allowance.
    ProviderBudgetExceeded,
    /// A sampled advective timestep guard failed.
    AdvectiveLimit,
}
