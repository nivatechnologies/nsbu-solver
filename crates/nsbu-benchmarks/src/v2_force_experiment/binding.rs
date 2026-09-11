//! Bitwise provenance binding for spectral force-resolution reports.
use super::{ForceFamily, ForceFamilyPlan, ForceFamilySettings, ForceRefinementSample};
use crate::{
    v2_experiment::{FamilyPlan, V2Family},
    v2_run, CASE_SHA256,
};
use nsbu_solver::{diagnostics::comparison::BandComparison, domain::TickClock, SolverError};

/// This consumer reports spectral velocity norms only and never assigns review readiness.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForceResolutionStatus {
    /// Raw force-grid differences with exact baseline provenance, without a policy decision.
    DiagnosticOnly,
}

/// A failed binding publishes no report and permanently terminates the consumer.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ForceBindingError {
    /// The two immutable plans do not describe one matching baseline trajectory.
    InvalidPlan,
    /// Clock, identity, settings, or coefficient words did not match at measurement time.
    InvalidBinding,
    /// Checked storage or work admission failed.
    Numerical(SolverError),
    /// A previous attempt failed.
    Terminated,
}
impl From<SolverError> for ForceBindingError {
    fn from(value: SolverError) -> Self {
        Self::Numerical(value)
    }
}

/// Fixed storage and complete finite bit-comparison work.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ForceBindingBounds {
    /// Consumer-owned constant storage.
    pub storage_bytes: usize,
    /// Force family, plain ordinary family, and binder; not a complete coordinator.
    pub joint_storage_bytes: usize,
    /// Maximum report attempts.
    pub attempts: usize,
    /// Real/imaginary words compared across the matching baseline states.
    pub coefficient_words: usize,
}

/// Admission binding one force-family branch to one ordinary-family branch.
#[derive(Debug, Clone, Copy)]
pub struct ForceBindingPlan<'a> {
    force: ForceFamilyPlan<'a>,
    ordinary: FamilyPlan<'a>,
    force_branch: usize,
    ordinary_branch: usize,
    words_per_attempt: usize,
    bounds: ForceBindingBounds,
}
impl<'a> ForceBindingPlan<'a> {
    /// Require identical manifests and a bit-comparable numerical baseline before allocation.
    pub fn new(
        force: ForceFamilyPlan<'a>,
        ordinary: FamilyPlan<'a>,
        force_branch: usize,
        ordinary_branch: usize,
        joint_cap: usize,
    ) -> Result<Self, ForceBindingError> {
        let force_plan = force
            .branch_plan(force_branch)
            .ok_or(ForceBindingError::InvalidPlan)?;
        let ordinary_plan = ordinary
            .branch_plan(ordinary_branch)
            .ok_or(ForceBindingError::InvalidPlan)?;
        if force.times().as_slice() != ordinary.times().as_slice()
            || !same_settings(force_plan.settings(), ordinary_plan.settings())
        {
            return Err(ForceBindingError::InvalidPlan);
        }
        let words_per_attempt = force_plan
            .resources()
            .domain()
            .layout()
            .half_len()
            .checked_mul(6)
            .ok_or(SolverError::SizeOverflow)?;
        let words = words_per_attempt
            .checked_mul(force.times().as_slice().len())
            .ok_or(SolverError::SizeOverflow)?;
        let storage_bytes = std::mem::size_of::<ForceBindingWorkspace<'_>>()
            .checked_add(std::mem::size_of::<SpectralForceResolutionSample>())
            .ok_or(SolverError::SizeOverflow)?;
        let joint_storage_bytes = force
            .bounds()
            .storage_bytes
            .checked_add(ordinary.bounds().storage_bytes)
            .and_then(|n| n.checked_add(storage_bytes))
            .ok_or(SolverError::SizeOverflow)?;
        if joint_storage_bytes > joint_cap {
            return Err(SolverError::ResourceLimit.into());
        }
        Ok(Self {
            force,
            ordinary,
            force_branch,
            ordinary_branch,
            words_per_attempt,
            bounds: ForceBindingBounds {
                storage_bytes,
                joint_storage_bytes,
                attempts: force.times().as_slice().len(),
                coefficient_words: words,
            },
        })
    }
    /// Complete consumer and joint-owner admission.
    pub fn bounds(self) -> ForceBindingBounds {
        self.bounds
    }
    /// Force-grid refinement settings retained in every successful report.
    pub fn force_settings(self) -> ForceFamilySettings {
        self.force.settings()
    }
    /// `(force-family branch, ordinary-family branch)` baseline slots.
    pub fn baseline_slots(self) -> (usize, usize) {
        (self.force_branch, self.ordinary_branch)
    }
}

/// Raw spectral force-grid differences after exact baseline-state validation.
#[derive(Debug, Clone, Copy)]
pub struct SpectralForceResolutionSample {
    clock: TickClock,
    comparisons: [BandComparison; 2],
    force_identity: [u8; 32],
    ordinary_identity: [u8; 32],
    settings: ForceFamilySettings,
    slots: (usize, usize),
}
impl SpectralForceResolutionSample {
    /// Exact accepted clock shared by all compared and bound states.
    pub fn clock(self) -> TickClock {
        self.clock
    }
    /// Raw M0/M1 and M1/M2 complete spectral-band comparisons.
    pub fn comparisons(self) -> [BandComparison; 2] {
        self.comparisons
    }
    /// Identity of the independently evolved force family.
    pub fn force_identity(self) -> [u8; 32] {
        self.force_identity
    }
    /// Identity of the independently evolved ordinary family.
    pub fn ordinary_identity(self) -> [u8; 32] {
        self.ordinary_identity
    }
    /// Exact force-family numerical profile.
    pub fn force_settings(self) -> ForceFamilySettings {
        self.settings
    }
    /// Exact matching baseline slots.
    pub fn baseline_slots(self) -> (usize, usize) {
        self.slots
    }
    /// Frozen exact-v2 case hash.
    pub fn case_sha256(self) -> &'static str {
        CASE_SHA256
    }
    /// Permanent diagnostic status; no generic observable units are inferred.
    pub fn status(self) -> ForceResolutionStatus {
        ForceResolutionStatus::DiagnosticOnly
    }
}

/// Constant-storage transactional binder over two caller-owned families.
pub struct ForceBindingWorkspace<'a> {
    plan: ForceBindingPlan<'a>,
    next: usize,
    attempted: usize,
    compared_words: usize,
    failed: bool,
    current: Option<SpectralForceResolutionSample>,
}
impl<'a> ForceBindingWorkspace<'a> {
    /// Construct without allocating either independently owned numerical family.
    pub fn new(plan: ForceBindingPlan<'a>) -> Self {
        Self {
            plan,
            next: 0,
            attempted: 0,
            compared_words: 0,
            failed: false,
            current: None,
        }
    }
    /// Most recent complete report, retained after a later terminal failure.
    pub fn current(&self) -> Option<SpectralForceResolutionSample> {
        self.current
    }
    /// `(attempted reports, compared coefficient words)` including failed attempts.
    pub fn charged_work(&self) -> (usize, usize) {
        (self.attempted, self.compared_words)
    }
    /// Bind one raw refinement report to the independently evolved ordinary baseline.
    pub fn measure(
        &mut self,
        force: &ForceFamily<'_>,
        ordinary: &V2Family<'_>,
        sample: ForceRefinementSample,
    ) -> Result<SpectralForceResolutionSample, ForceBindingError> {
        if self.failed {
            return Err(ForceBindingError::Terminated);
        }
        if self.attempted == self.plan.bounds.attempts {
            self.failed = true;
            return Err(SolverError::ProviderBudgetExceeded.into());
        }
        self.attempted += 1;
        self.compared_words = self
            .compared_words
            .checked_add(self.plan.words_per_attempt)
            .ok_or(SolverError::SizeOverflow)?;
        let result = self.bind(force, ordinary, sample);
        match result {
            Ok(report) => {
                self.next += 1;
                self.current = Some(report);
                Ok(report)
            }
            Err(error) => {
                self.failed = true;
                Err(error)
            }
        }
    }
    fn bind(
        &mut self,
        force: &ForceFamily<'_>,
        ordinary: &V2Family<'_>,
        sample: ForceRefinementSample,
    ) -> Result<SpectralForceResolutionSample, ForceBindingError> {
        let expected = self.require_publications(force, ordinary)?;
        self.require_report(sample, expected)?;
        let (left, right) = self.require_baseline(force, ordinary, expected)?;
        same_coefficients(left.state(), right.state())?;
        Ok(SpectralForceResolutionSample {
            clock: expected,
            comparisons: sample.comparisons(),
            force_identity: self.plan.force.identity(),
            ordinary_identity: self.plan.ordinary.identity(),
            settings: self.plan.force.settings(),
            slots: self.plan.baseline_slots(),
        })
    }
    fn require_publications(
        &self,
        force: &ForceFamily<'_>,
        ordinary: &V2Family<'_>,
    ) -> Result<TickClock, ForceBindingError> {
        let expected = self
            .plan
            .force
            .require_sample(force, self.next)
            .map_err(|_| ForceBindingError::InvalidBinding)?;
        let ordinary_clock = self
            .plan
            .ordinary
            .require_sample(ordinary, self.next)
            .map_err(|_| ForceBindingError::InvalidBinding)?;
        if ordinary_clock != expected {
            return Err(ForceBindingError::InvalidBinding);
        }
        Ok(expected)
    }
    fn require_report(
        &self,
        sample: ForceRefinementSample,
        expected: TickClock,
    ) -> Result<(), ForceBindingError> {
        if sample.identity() != self.plan.force.identity() || sample.clock() != expected {
            return Err(ForceBindingError::InvalidBinding);
        }
        Ok(())
    }
    fn require_baseline<'b>(
        &self,
        force: &'b ForceFamily<'_>,
        ordinary: &'b V2Family<'_>,
        expected: TickClock,
    ) -> Result<(&'b v2_run::Run, &'b v2_run::Run), ForceBindingError> {
        let left = force
            .branch(self.plan.force_branch)
            .ok_or(ForceBindingError::InvalidBinding)?;
        let right = ordinary
            .branch(self.plan.ordinary_branch)
            .ok_or(ForceBindingError::InvalidBinding)?;
        if left.state().clock() != expected
            || right.state().clock() != expected
            || !same_settings(left.plan().settings(), right.plan().settings())
        {
            return Err(ForceBindingError::InvalidBinding);
        }
        Ok((left, right))
    }
}

fn same_settings(a: v2_run::Settings, b: v2_run::Settings) -> bool {
    let ac = a.configuration;
    let bc = b.configuration;
    a.domain == b.domain
        && a.force == b.force
        && a.initial_clock == b.initial_clock
        && ac.method == bc.method
        && ac.limits.endpoint == bc.limits.endpoint
        && ac.limits.step_ticks == bc.limits.step_ticks
        && ac.limits.maximum_attempts == bc.limits.maximum_attempts
        && words2(ac.tolerances.absolute) == words2(bc.tolerances.absolute)
        && words2(ac.tolerances.relative) == words2(bc.tolerances.relative)
        && a.advective_limit.to_bits() == b.advective_limit.to_bits()
}
fn words2(values: [f64; 2]) -> [u64; 2] {
    values.map(f64::to_bits)
}

fn same_coefficients(
    left: &nsbu_solver::domain::SpectralState,
    right: &nsbu_solver::domain::SpectralState,
) -> Result<(), ForceBindingError> {
    if left.plan().domain() != right.plan().domain() {
        return Err(ForceBindingError::InvalidBinding);
    }
    for axis in 0..3 {
        let a = left.component(axis)?;
        let b = right.component(axis)?;
        same_values(a, b)?;
    }
    Ok(())
}

fn same_values(
    left: &[nsbu_solver::Complex64],
    right: &[nsbu_solver::Complex64],
) -> Result<(), ForceBindingError> {
    if left.len() != right.len() {
        return Err(ForceBindingError::InvalidBinding);
    }
    for (x, y) in left.iter().zip(right) {
        if (x.re.to_bits(), x.im.to_bits()) != (y.re.to_bits(), y.im.to_bits()) {
            return Err(ForceBindingError::InvalidBinding);
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use nsbu_solver::Complex64;

    #[test]
    fn one_word_corruption_and_length_change_fail_bitwise_binding() {
        let exact = [Complex64::new(1.0, -2.0), Complex64::new(0.0, 0.0)];
        assert_eq!(same_values(&exact, &exact), Ok(()));
        let mut corrupt = exact;
        corrupt[1].im = f64::from_bits(1);
        assert_eq!(
            same_values(&exact, &corrupt),
            Err(ForceBindingError::InvalidBinding)
        );
        assert_eq!(
            same_values(&exact, &exact[..1]),
            Err(ForceBindingError::InvalidBinding)
        );
    }
}
