//! Transactional construction and allocation-free strict-band copies.
use super::{SharedForceBinding, SharedForceTablePlan, SharedForceWork};
use crate::runtime_force::ForceSettings;
use nsbu_solver::{
    domain::{Domain, TickClock},
    integrators::forcing::PrescribedForce,
    spectral::transfer,
    Complex64, SolverError,
};

/// Refusals from immutable table construction or a bound copy stream.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum SharedForceError {
    /// The table or copy request failed a checked numerical/resource contract.
    Numerical(SolverError),
    /// The supplied binding does not describe this table and retained domain.
    ForeignBinding,
    /// The clock is absent or its admitted per-domain copies are exhausted.
    UnexpectedRequest,
    /// A prior copy attempt failed permanently.
    Terminated,
}
impl From<SolverError> for SharedForceError {
    fn from(error: SolverError) -> Self {
        Self::Numerical(error)
    }
}
impl SharedForceError {
    /// Preserve deterministic table refusal categories at a generic force-provider boundary.
    pub fn solver_error(self) -> SolverError {
        match self {
            Self::Numerical(error) => error,
            Self::ForeignBinding => SolverError::InvalidPayload,
            Self::UnexpectedRequest => SolverError::InvalidClock,
            Self::Terminated => SolverError::ProviderBudgetExceeded,
        }
    }
}

/// One complete table copy; it is a diagnostic work record, not an acceptance result.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SharedForceCopy {
    clock: TickClock,
    domain: Domain,
    identity: [u8; 32],
    settings: ForceSettings,
    charged: SharedForceWork,
}
impl SharedForceCopy {
    /// Exact clock of the copied immutable entry.
    pub fn clock(self) -> TickClock {
        self.clock
    }
    /// Exact retained domain requested by the bound consumer.
    pub fn domain(self) -> Domain {
        self.domain
    }
    /// Full table/provider/manifest identity.
    pub fn identity(self) -> [u8; 32] {
        self.identity
    }
    /// Original-force M and worker selection bound to the copy.
    pub fn settings(self) -> ForceSettings {
        self.settings
    }
    /// Conservative work charged for this complete copy attempt.
    pub fn charged_work(self) -> SharedForceWork {
        self.charged
    }
}

pub(super) struct StoredClock {
    clock: TickClock,
    remaining: [usize; 3],
    values: [Vec<Complex64>; 3],
}

pub(super) const fn stored_clock_size() -> usize {
    std::mem::size_of::<StoredClock>()
}

/// One eagerly constructed immutable force slab with bounded mutable copy ledgers.
pub struct SharedForceTable<'a> {
    plan: SharedForceTablePlan<'a>,
    slots: Vec<StoredClock>,
    charged: SharedForceWork,
    current: Option<SharedForceCopy>,
    failed: bool,
}

impl<'a> SharedForceTable<'a> {
    /// Build every declared clock once on the maximum domain before publishing the owner.
    pub fn new(plan: SharedForceTablePlan<'a>) -> Result<Self, SharedForceError> {
        let provider_limits = plan.provider();
        let mut provider = plan
            .settings()
            .build(plan.domains()[2], provider_limits.storage_bytes)?;
        let mut slots = Vec::new();
        slots
            .try_reserve_exact(plan.manifest().len())
            .map_err(|_| SolverError::AllocationFailed)?;
        for request in plan.manifest() {
            let mut slot = stored(*request, plan.domains()[2])?;
            let report = provider.evaluate(
                request.clock(),
                provider_limits,
                slot.values.each_mut().map(Vec::as_mut_slice),
            )?;
            if report.work_units > provider_limits.work_units
                || report.scalar_transforms > provider_limits.scalar_transforms
            {
                return Err(SolverError::ProviderBudgetExceeded.into());
            }
            slots.push(slot);
        }
        Ok(Self {
            plan,
            slots,
            charged: SharedForceWork {
                provider_evaluations: plan.manifest().len(),
                provider_work_units: plan.bounds().work.provider_work_units,
                scalar_transforms: plan.bounds().work.scalar_transforms,
                ..SharedForceWork::default()
            },
            current: None,
            failed: false,
        })
    }

    /// Immutable admission used by this owner.
    pub fn plan(&self) -> SharedForceTablePlan<'a> {
        self.plan
    }
    /// Last complete copy; a later failure preserves this record.
    pub fn current(&self) -> Option<SharedForceCopy> {
        self.current
    }
    /// Construction plus conservatively charged copy attempts.
    pub fn charged_work(&self) -> SharedForceWork {
        self.charged
    }
    /// Whether a failed attempt permanently stopped this copy stream.
    pub fn is_terminated(&self) -> bool {
        self.failed
    }
    /// Remaining successful copies for an exact clock and admitted domain.
    pub fn remaining(&self, clock: TickClock, domain: Domain) -> Option<usize> {
        let index = self.domain_index(domain)?;
        self.slots
            .iter()
            .find(|slot| slot.clock == clock)
            .map(|slot| slot.remaining[index])
    }

    /// Copy one admitted strict band after charging a whole attempt and validating all inputs.
    pub fn copy(
        &mut self,
        binding: SharedForceBinding,
        clock: TickClock,
        output: [&mut [Complex64]; 3],
    ) -> Result<SharedForceCopy, SharedForceError> {
        if self.failed {
            return Err(SharedForceError::Terminated);
        }
        if self.charged.copy_attempts == self.plan.maximum_attempts() {
            self.failed = true;
            return Err(SharedForceError::Numerical(SolverError::ResourceLimit));
        }
        let attempt = self.charge_attempt();
        match self.copy_checked(binding, clock, output, attempt) {
            Ok(report) => {
                self.current = Some(report);
                Ok(report)
            }
            Err(error) => {
                self.failed = true;
                Err(error)
            }
        }
    }

    fn charge_attempt(&mut self) -> SharedForceWork {
        let allowance = self.plan.bounds().work;
        let per_attempt = SharedForceWork {
            copy_attempts: 1,
            clock_comparisons: self.slots.len(),
            binding_checks: allowance.binding_checks / allowance.copy_attempts,
            coefficient_words_copied: allowance.coefficient_words_copied / allowance.copy_attempts,
            transfer_visits: allowance.transfer_visits / allowance.copy_attempts,
            ..SharedForceWork::default()
        };
        self.charged.copy_attempts += 1;
        self.charged.clock_comparisons += per_attempt.clock_comparisons;
        self.charged.binding_checks += per_attempt.binding_checks;
        self.charged.coefficient_words_copied += per_attempt.coefficient_words_copied;
        self.charged.transfer_visits += per_attempt.transfer_visits;
        per_attempt
    }

    fn copy_checked(
        &mut self,
        binding: SharedForceBinding,
        clock: TickClock,
        output: [&mut [Complex64]; 3],
        charged: SharedForceWork,
    ) -> Result<SharedForceCopy, SharedForceError> {
        let index = self.validate(binding, &output)?;
        let slot = self
            .slots
            .iter_mut()
            .find(|slot| slot.clock == clock)
            .ok_or(SharedForceError::UnexpectedRequest)?;
        if slot.remaining[index] == 0 {
            return Err(SharedForceError::UnexpectedRequest);
        }
        let source = self.plan.domains()[2].layout();
        let target = binding.domain.layout();
        for (input, output) in slot.values.iter().zip(output) {
            transfer(source, target, input, output)?;
        }
        slot.remaining[index] -= 1;
        Ok(SharedForceCopy {
            clock,
            domain: binding.domain,
            identity: binding.identity,
            settings: binding.settings,
            charged,
        })
    }

    fn validate(
        &self,
        binding: SharedForceBinding,
        output: &[&mut [Complex64]; 3],
    ) -> Result<usize, SharedForceError> {
        if binding.identity != self.plan.identity() || binding.settings != self.plan.settings() {
            return Err(SharedForceError::ForeignBinding);
        }
        let index = self
            .domain_index(binding.domain)
            .ok_or(SharedForceError::ForeignBinding)?;
        if output
            .iter()
            .any(|values| values.len() != binding.domain.layout().half_len())
        {
            return Err(SolverError::InvalidPayload.into());
        }
        Ok(index)
    }

    fn domain_index(&self, domain: Domain) -> Option<usize> {
        self.plan
            .domains()
            .iter()
            .position(|candidate| *candidate == domain)
    }
}

fn stored(
    request: super::SharedForceClock,
    domain: Domain,
) -> Result<StoredClock, SharedForceError> {
    let length = domain.layout().half_len();
    Ok(StoredClock {
        clock: request.clock(),
        remaining: request.maximum_copies(),
        values: [values(length)?, values(length)?, values(length)?],
    })
}

fn values(length: usize) -> Result<Vec<Complex64>, SharedForceError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(length)
        .map_err(|_| SolverError::AllocationFailed)?;
    values.resize(length, Complex64::new(0.0, 0.0));
    Ok(values)
}
