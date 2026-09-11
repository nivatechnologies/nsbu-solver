//! Transactional accepted-node reconstruction backed by the exact-v2 force provider.

use super::{
    reconstruction::{ReconstructionObserver, ReconstructionObserverLimits},
    v2::V2Observer,
};
use crate::{runtime_force::ForceSettings, runtime_force::RunForce};
use nsbu_solver::{domain::ResourcePlan, domain::SpectralState, SolverError};

/// Exact-v2 reconstruction observer with its own prescribed-force and product scratch.
pub type V2ReconstructionObserver = ReconstructionObserver<RunForce>;

impl ReconstructionObserver<RunForce> {
    /// Declare full balance, four-node and finite initial-rest/endpoint work.
    pub fn v2_limits(
        source: nsbu_solver::domain::Domain,
        settings: ForceSettings,
        samples: usize,
    ) -> Result<ReconstructionObserverLimits, SolverError> {
        let balance = V2Observer::limits(source, settings, samples)?;
        Self::limits_for(source, samples, balance)
    }

    /// Allocate an exact-v2 observer and independently capture the actual rest endpoint.
    pub fn new_v2(
        plan: ResourcePlan,
        settings: ForceSettings,
        samples: usize,
        from_rest: &SpectralState,
    ) -> Result<Self, SolverError> {
        let limits = Self::v2_limits(plan.domain(), settings, samples)?;
        Self::admit(plan, from_rest, limits)?;
        let balance = V2Observer::new(plan.domain(), settings, samples, plan.classes()[6])?;
        Self::new_owned(plan, from_rest, limits, balance)
    }
}
