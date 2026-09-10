//! Explicit channel allocations prevent missing uncertainty from disappearing into zero.
use super::{
    refinement::{positive, Requirement, Rule},
    VerificationError,
};

/// Mandatory empirical comparison channels; trajectory and arithmetic identities are checked separately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(usize)]
pub enum Channel {
    /// Full-band comparison on three increasing retained grids.
    Space,
    /// Three temporal settings on the finest retained grid.
    Time,
    /// Independent CM/HO comparison.
    Method,
    /// Increased prescribed-force sampling resolution, retaining the full comparison band.
    ForceResolution,
    /// Increased prescribed-force evaluation precision.
    ForcePrecision,
    /// Independently improved integrator/FFT/reduction arithmetic on the current grid.
    Arithmetic,
    /// Independent reference precision refinement.
    ReferencePrecision,
    /// Transfer versus direct fine evolution; unused transfer needs separate lineage confirmation.
    Transfer,
    /// Refinement of physical-space diagnostic sampling.
    Sampling,
    /// Off-stage reconstruction refinement at unchanged exact probe times.
    Reconstruction,
    /// Independently refined balance/region quadrature.
    Quadrature,
}

/// Stable channel order in frozen policies and measurement records.
pub const CHANNELS: [Channel; 11] = [
    Channel::Space,
    Channel::Time,
    Channel::Method,
    Channel::ForceResolution,
    Channel::ForcePrecision,
    Channel::Arithmetic,
    Channel::ReferencePrecision,
    Channel::Transfer,
    Channel::Sampling,
    Channel::Reconstruction,
    Channel::Quadrature,
];
impl Channel {
    /// These channels need three settings or a separately supported subordinate floor.
    pub fn requires_refinement(self) -> bool {
        matches!(
            self,
            Self::Space | Self::Time | Self::Sampling | Self::Reconstruction | Self::Quadrature
        )
    }
}

/// Immutable allocations for one observable, in one declared set of units.
/// Comparison deltas remain empirical measurements; this ledger is not a PDE error enclosure.
#[derive(Debug, Clone, Copy)]
pub struct Budget {
    total: f64,
    allocated: f64,
    rules: [Rule; 11],
}
impl Budget {
    /// Admit positive rules for every channel and conservatively sum their allocations.
    /// Even unused transfer retains its allocation; provenance cannot be replaced by a zero budget.
    pub fn new(total: f64, rules: [Rule; 11]) -> Result<Self, VerificationError> {
        positive(total)?;
        let mut allocated = 0.0;
        for (channel, rule) in CHANNELS.into_iter().zip(rules) {
            if channel.requires_refinement() && rule.requirement() != Requirement::Refinement {
                return Err(VerificationError::InvalidRequirement);
            }
            allocated = upper_add(allocated, rule.budget())?;
        }
        if allocated > total {
            return Err(VerificationError::ExcessAllocation);
        }
        Ok(Self {
            total,
            allocated,
            rules,
        })
    }

    /// Frozen total tolerance against which the independent reference discrepancy is checked.
    pub fn total(self) -> f64 {
        self.total
    }

    /// Upward-rounded bookkeeping bound on the sum of declared allocations.
    pub fn allocated(self) -> f64 {
        self.allocated
    }

    /// The admitted immutable rule for a named channel.
    pub fn rule(self, channel: Channel) -> Rule {
        self.rules[channel as usize]
    }
}

// TwoSum's residual detects a downward-rounded positive addition. Each accumulated
// value is already an upper bound; only downward rounding needs a next-up correction.
fn upper_add(a: f64, b: f64) -> Result<f64, VerificationError> {
    let sum = a + b;
    if !sum.is_finite() {
        return Err(VerificationError::ExcessAllocation);
    }
    let recovered = sum - a;
    let error = (a - (sum - recovered)) + (b - recovered);
    Ok(if error > 0.0 { sum.next_up() } else { sum })
}
