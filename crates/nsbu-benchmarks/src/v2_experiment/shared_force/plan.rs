//! Admission and complete resource bounds for one force-table slab.
use crate::runtime_force::ForceSettings;
use nsbu_solver::{
    domain::{Domain, TickClock},
    integrators::forcing::ForceLimits,
    SolverError,
};

const ALLOCATION_ALLOWANCE: usize = 64;

/// One exact slab clock and bounded successful copies for the three retained domains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SharedForceClock {
    clock: TickClock,
    maximum_copies: [usize; 3],
}
impl SharedForceClock {
    /// Bind one complete exact clock and positive per-domain copy allowances.
    pub fn new(clock: TickClock, maximum_copies: [usize; 3]) -> Result<Self, SolverError> {
        if maximum_copies.contains(&0) {
            return Err(SolverError::InvalidPayload);
        }
        Ok(Self {
            clock,
            maximum_copies,
        })
    }
    /// Complete exact clock, including exponent, target, elapsed and remaining ticks.
    pub fn clock(self) -> TickClock {
        self.clock
    }
    /// Successful copy allowances in the admitted domain order.
    pub fn maximum_copies(self) -> [usize; 3] {
        self.maximum_copies
    }
}

/// Complete construction and runtime work bounds.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SharedForceWork {
    /// Calls charged before request validation, including terminal failures.
    pub copy_attempts: usize,
    /// Original exact-v2 evaluations used to build the immutable table.
    pub provider_evaluations: usize,
    /// Original-provider work units charged during construction.
    pub provider_work_units: usize,
    /// Scalar transforms charged during construction.
    pub scalar_transforms: usize,
    /// Maximum exact-clock lookup comparisons.
    pub clock_comparisons: usize,
    /// Conservatively charged copied coefficient words.
    pub coefficient_words_copied: usize,
}

/// Persistent, construction-peak and caller-owned reservations.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct SharedForceBounds {
    /// Storage owned after the construction provider is dropped.
    pub storage_bytes: usize,
    /// Borrowed manifest bytes retained by the plan and table.
    pub caller_manifest_bytes: usize,
    /// Largest caller output admitted for one copy.
    pub caller_output_bytes: usize,
    /// Peak owner plus provider storage during transactional construction.
    pub construction_peak_bytes: usize,
    /// Maximum simultaneous owner, borrowed-manifest, provider/output storage.
    pub joint_peak_bytes: usize,
    /// Complete work bound, including every admitted copy attempt.
    pub work: SharedForceWork,
}

/// Opaque binding of the full table identity, force settings and one retained domain.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SharedForceBinding {
    pub(super) identity: [u8; 32],
    pub(super) settings: ForceSettings,
    pub(super) domain: Domain,
}

/// Borrowed immutable admission for one eagerly built integration-force slab.
#[derive(Debug, Clone, Copy)]
pub struct SharedForceTablePlan<'a> {
    settings: ForceSettings,
    domains: [Domain; 3],
    manifest: &'a [SharedForceClock],
    maximum_attempts: usize,
    provider: ForceLimits,
    bounds: SharedForceBounds,
    identity: [u8; 32],
}

impl<'a> SharedForceTablePlan<'a> {
    /// Validate the full provider/domain/clock manifest and its joint cap.
    pub fn new(
        settings: ForceSettings,
        domains: [Domain; 3],
        manifest: &'a [SharedForceClock],
        maximum_attempts: usize,
        joint_cap: usize,
    ) -> Result<Self, SolverError> {
        validate_domains(domains)?;
        let planned_copies = validate_manifest(manifest)?;
        if maximum_attempts == 0 || maximum_attempts < planned_copies {
            return Err(SolverError::ResourceLimit);
        }
        let provider = settings.limits(domains[2])?;
        let mut plan = Self {
            settings,
            domains,
            manifest,
            maximum_attempts,
            provider,
            bounds: SharedForceBounds::default(),
            identity: [0; 32],
        };
        plan.bounds = bounds(&plan)?;
        if plan.bounds.joint_peak_bytes > joint_cap {
            return Err(SolverError::ResourceLimit);
        }
        plan.identity = super::identity::compute(&plan);
        Ok(plan)
    }
    /// Original-force sampling grid and worker selection.
    pub fn settings(self) -> ForceSettings {
        self.settings
    }
    /// Strictly nested retained domains; the third is the table domain.
    pub fn domains(self) -> [Domain; 3] {
        self.domains
    }
    /// Complete ordered slab manifest retained in caller-owned storage.
    pub fn manifest(self) -> &'a [SharedForceClock] {
        self.manifest
    }
    /// Maximum calls, including malformed or otherwise failed attempts.
    pub fn maximum_attempts(self) -> usize {
        self.maximum_attempts
    }
    /// Complete preflight storage and work bounds.
    pub fn bounds(self) -> SharedForceBounds {
        self.bounds
    }
    /// Canonical identity binding case, provider, domains, attempts and manifest.
    pub fn identity(self) -> [u8; 32] {
        self.identity
    }
    /// Create the only valid caller binding for one admitted domain.
    pub fn binding(self, index: usize) -> Option<SharedForceBinding> {
        self.domains
            .get(index)
            .copied()
            .map(|domain| SharedForceBinding {
                identity: self.identity,
                settings: self.settings,
                domain,
            })
    }
    pub(super) fn provider(self) -> ForceLimits {
        self.provider
    }
}

fn validate_domains(domains: [Domain; 3]) -> Result<(), SolverError> {
    for pair in domains.windows(2) {
        let a = pair[0].layout().dimensions();
        let b = pair[1].layout().dimensions();
        if a.into_iter().zip(b).any(|(left, right)| left >= right) {
            return Err(SolverError::InvalidDomain);
        }
    }
    let lengths = domains[2].lengths().map(f64::to_bits);
    let viscosity = domains[2].viscosity().to_bits();
    if domains.iter().any(|domain| {
        domain.lengths().map(f64::to_bits) != lengths || domain.viscosity().to_bits() != viscosity
    }) {
        return Err(SolverError::InvalidDomain);
    }
    Ok(())
}

fn validate_manifest(manifest: &[SharedForceClock]) -> Result<usize, SolverError> {
    let first = manifest.first().ok_or(SolverError::InvalidPayload)?;
    let mut previous = None;
    let mut copies = 0usize;
    for request in manifest {
        let clock = request.clock;
        if clock.exponent() != first.clock.exponent()
            || clock.target() != first.clock.target()
            || previous.is_some_and(|elapsed| clock.elapsed() <= elapsed)
            || request.maximum_copies.contains(&0)
            || crate::time::BenchmarkTime::new(clock).is_err()
        {
            return Err(SolverError::InvalidClock);
        }
        previous = Some(clock.elapsed());
        for count in request.maximum_copies {
            copies = copies.checked_add(count).ok_or(SolverError::SizeOverflow)?;
        }
    }
    Ok(copies)
}

fn bounds(plan: &SharedForceTablePlan<'_>) -> Result<SharedForceBounds, SolverError> {
    let storage = storage_bounds(plan)?;
    Ok(SharedForceBounds {
        storage_bytes: storage.owner,
        caller_manifest_bytes: storage.manifest,
        caller_output_bytes: storage.output,
        construction_peak_bytes: storage.construction,
        joint_peak_bytes: storage.joint,
        work: work_bounds(plan, storage.words)?,
    })
}

struct StorageBounds {
    owner: usize,
    manifest: usize,
    output: usize,
    construction: usize,
    joint: usize,
    words: usize,
}

fn storage_bounds(plan: &SharedForceTablePlan<'_>) -> Result<StorageBounds, SolverError> {
    let clocks = plan.manifest.len();
    let half = plan.domains[2].layout().half_len();
    let words = multiply(half, 3)?;
    let payload = multiply(
        multiply(words, clocks)?,
        std::mem::size_of::<nsbu_solver::Complex64>(),
    )?;
    let slot_metadata = multiply(clocks, super::table::stored_clock_size())?;
    let allocations = multiply(add(multiply(clocks, 3)?, 1)?, ALLOCATION_ALLOWANCE)?;
    let owner = sum(&[
        std::mem::size_of::<super::SharedForceTable<'_>>(),
        payload,
        slot_metadata,
        allocations,
    ])?;
    let manifest = multiply(clocks, std::mem::size_of::<SharedForceClock>())?;
    let output = multiply(words, std::mem::size_of::<nsbu_solver::Complex64>())?;
    let construction = add(owner, plan.provider.storage_bytes)?;
    Ok(StorageBounds {
        owner,
        manifest,
        output,
        construction,
        joint: add(construction, manifest)?.max(sum(&[owner, manifest, output])?),
        words,
    })
}

fn work_bounds(
    plan: &SharedForceTablePlan<'_>,
    words: usize,
) -> Result<SharedForceWork, SolverError> {
    let clocks = plan.manifest.len();
    Ok(SharedForceWork {
        copy_attempts: plan.maximum_attempts,
        provider_evaluations: clocks,
        provider_work_units: multiply(plan.provider.work_units, clocks)?,
        scalar_transforms: multiply(plan.provider.scalar_transforms, clocks)?,
        clock_comparisons: multiply(clocks, plan.maximum_attempts)?,
        coefficient_words_copied: multiply(words, plan.maximum_attempts)?,
    })
}

fn multiply(left: usize, right: usize) -> Result<usize, SolverError> {
    left.checked_mul(right).ok_or(SolverError::SizeOverflow)
}

fn add(left: usize, right: usize) -> Result<usize, SolverError> {
    left.checked_add(right).ok_or(SolverError::SizeOverflow)
}

fn sum(values: &[usize]) -> Result<usize, SolverError> {
    values
        .iter()
        .try_fold(0usize, |total, value| add(total, *value))
}
