//! Fixed-step controller state and complete per-attempt numerical outcomes.
use crate::{
    domain::TickClock,
    integrators::{
        indicator::{Indicators, Tolerances},
        method::Method,
        trajectory::{RunLimits, StopReason},
    },
    SolverError,
};

/// Frozen fixed-step configuration; a changed configuration requires a separately recorded run.
#[derive(Debug, Clone, Copy)]
pub struct Configuration {
    /// Exact endpoint, macro step and finite attempt limit.
    pub limits: RunLimits,
    /// Independent method selection.
    pub method: Method,
    /// Raw full/two-half-step discrepancy budgets.
    pub tolerances: Tolerances,
}

/// Outcome retained even when physical state does not advance.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum Outcome {
    /// Physical and required diagnostic proposals were committed together.
    Committed(Indicators),
    /// A completed attempt failed its empirical local discrepancy test.
    Rejected(Indicators),
    /// Provider, arithmetic or diagnostic refusal; retain indicators if they were computed.
    Refused {
        /// Structured cause at the point of refusal.
        cause: SolverError,
        /// Present for a diagnostic refusal following a completed integration attempt.
        indicators: Option<Indicators>,
    },
}
impl Outcome {
    /// Raw channels, retained for both commits and failures whenever evaluation completed.
    pub fn indicators(self) -> Option<Indicators> {
        match self {
            Self::Committed(value) | Self::Rejected(value) => Some(value),
            Self::Refused { indicators, .. } => indicators,
        }
    }
    fn validate(self) -> Result<(), SolverError> {
        match self {
            Self::Committed(indicators) => {
                validate_indicators(indicators)?;
                if indicators.ratios.iter().any(|r| *r > 1.0) {
                    return Err(SolverError::InvalidPayload);
                }
            }
            Self::Rejected(indicators) => {
                validate_indicators(indicators)?;
                if indicators.ratios.iter().all(|r| *r <= 1.0) {
                    return Err(SolverError::InvalidPayload);
                }
            }
            Self::Refused {
                indicators: Some(indicators),
                ..
            } => validate_indicators(indicators)?,
            Self::Refused {
                indicators: None, ..
            } => {}
        }
        Ok(())
    }
}

/// Copyable checkpoint component. It preserves limits, counts, clock and terminal status.
/// Controller state alone does not preserve the required per-attempt log or prove execution.
#[derive(Debug, Clone, Copy)]
pub struct Controller {
    configuration: Configuration,
    clock: TickClock,
    attempted: usize,
    committed: usize,
    stopped: Option<StopReason>,
    required: usize,
}
impl Controller {
    /// Admit the complete fixed run from exact zero before any physical attempt.
    pub fn new(clock: TickClock, configuration: Configuration) -> Result<Self, SolverError> {
        if clock.elapsed() != 0 {
            return Err(SolverError::InvalidClock);
        }
        configuration.tolerances.validate()?;
        let required = configuration
            .limits
            .attempts_for_method(clock, configuration.method)?;
        Ok(Self {
            configuration,
            clock,
            attempted: 0,
            committed: 0,
            stopped: None,
            required,
        })
    }
    /// Frozen limits and arithmetic method/tolerance choices.
    pub fn configuration(self) -> Configuration {
        self.configuration
    }
    /// Admitted number of commits to the endpoint, reused by diagnostic storage planning.
    pub fn required_commits(self) -> usize {
        self.required
    }
    /// Actual last committed clock.
    pub fn clock(self) -> TickClock {
        self.clock
    }
    /// All recorded attempts, including numerical rejections and refusals.
    pub fn attempted(self) -> usize {
        self.attempted
    }
    /// Actual committed advances, separate from attempted work.
    pub fn committed(self) -> usize {
        self.committed
    }
    /// Terminal reason; checkpoint restoration must not erase this status.
    pub fn stopped(self) -> Option<StopReason> {
        self.stopped
    }
    /// Remaining finite allowance; a terminal status still prohibits more attempts.
    pub fn attempts_left(self) -> usize {
        self.configuration.limits.maximum_attempts - self.attempted
    }
    /// Check the next requested interval without rounding or silently rescheduling it.
    pub fn next_clock(self) -> Result<TickClock, SolverError> {
        if self.stopped.is_some() {
            return Err(SolverError::RetryLimit);
        }
        Ok(self.clock.stages(self.configuration.limits.step_ticks)?[4])
    }
    /// Prepare the next controller value without changing the original on malformed evidence.
    /// The experiment coordinator must retain the same outcome in its complete attempt log.
    pub fn with_outcome(mut self, outcome: Outcome) -> Result<Self, SolverError> {
        let next = self.next_clock()?;
        outcome.validate()?;
        self.attempted += 1;
        match outcome {
            Outcome::Committed(_) => {
                self.clock = next;
                self.committed += 1;
                if next.elapsed() == self.configuration.limits.endpoint {
                    self.stopped = Some(StopReason::EndpointReached);
                }
            }
            Outcome::Rejected(_) => self.stopped = Some(StopReason::LocalErrorRejected),
            Outcome::Refused { cause, .. } => self.stopped = Some(StopReason::Refused(cause)),
        }
        Ok(self)
    }
}

fn validate_indicators(indicators: Indicators) -> Result<(), SolverError> {
    if indicators
        .errors
        .into_iter()
        .chain(indicators.ratios)
        .any(|value| !value.is_finite() || value < 0.0)
    {
        return Err(SolverError::InvalidPayload);
    }
    Ok(())
}
