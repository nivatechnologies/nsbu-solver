//! Checked storage and separately named work units for independent residual probes.
use super::{
    ConservativeWorkspace, CyclicSine, Domain, PrescribedForce, ResidualWorkspace, SolverError,
};

/// Charged worst-case attempt work, including malformed requests; units are not wall-clock time.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResidualWork {
    /// Consumed calls, including failures before force/product evaluation.
    pub probes: usize,
    /// Prescribed-force provider work units on the doubled grid.
    pub provider_work_units: usize,
    /// Provider transforms plus nine independent conservative-product scalar transforms per probe.
    pub scalar_transforms: usize,
    /// Weighted coefficient visits for reconstruction, validation and residual assembly, excluding FFTs.
    pub coefficient_work_units: usize,
}
/// Full fixed allocation and finite work admitted before any diagnostic buffer is allocated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResidualBounds {
    /// All element storage and owning headers; allocator overhead and the borrowed run are separate.
    pub storage_bytes: usize,
    /// Maximum admitted calls, without automatic retry or budget reset.
    pub maximum_probes: usize,
    /// Full worst-case charges across the entire probe allowance.
    pub work: ResidualWork,
}
impl ResidualWorkspace {
    /// Include six retained fields, ten double-grid fields and all independent product/FFT scratch.
    pub fn reservation(
        source: Domain,
        maximum_probes: usize,
    ) -> Result<ResidualBounds, SolverError> {
        if maximum_probes == 0 {
            return Err(SolverError::ResourceLimit);
        }
        let diagnostic = ConservativeWorkspace::diagnostic_domain(source)?;
        Ok(ResidualBounds {
            storage_bytes: storage(source, diagnostic)?,
            maximum_probes,
            work: work(source, diagnostic, maximum_probes)?,
        })
    }
}
// Storage and work are independent admission obligations; keep checked arithmetic in each ledger.
fn storage(source: Domain, diagnostic: Domain) -> Result<usize, SolverError> {
    let fields = add(
        mul(source.layout().half_len(), 6)?,
        mul(diagnostic.layout().half_len(), 10)?,
    )?;
    add(
        add(
            mul(fields, std::mem::size_of::<nsbu_solver::Complex64>())?,
            ConservativeWorkspace::reservation(source)?,
        )?,
        std::mem::size_of::<ResidualWorkspace>(),
    )
}
fn work(source: Domain, diagnostic: Domain, probes: usize) -> Result<ResidualWork, SolverError> {
    let force = CyclicSine::new(diagnostic)?
        .limits()
        .ok_or(SolverError::UnknownProviderCost)?;
    // Allow 64 retained and 64 double-grid visits per coefficient, six real-grid
    // tensor-product visits per point and 512 fixed geometry/weight units. FFT
    // internals are counted separately as transforms. These are weighted visits, not FLOPs.
    let visits = add(
        add(
            add(
                mul(source.layout().half_len(), 64)?,
                mul(diagnostic.layout().half_len(), 64)?,
            )?,
            mul(diagnostic.layout().real_len(), 6)?,
        )?,
        512,
    )?;
    Ok(ResidualWork {
        probes,
        provider_work_units: mul(force.work_units, probes)?,
        scalar_transforms: mul(add(force.scalar_transforms, 9)?, probes)?,
        coefficient_work_units: mul(visits, probes)?,
    })
}
fn add(a: usize, b: usize) -> Result<usize, SolverError> {
    a.checked_add(b).ok_or(SolverError::SizeOverflow)
}
fn mul(a: usize, b: usize) -> Result<usize, SolverError> {
    a.checked_mul(b).ok_or(SolverError::SizeOverflow)
}
