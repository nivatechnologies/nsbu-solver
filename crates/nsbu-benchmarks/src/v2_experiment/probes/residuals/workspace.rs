//! Fresh doubled-band exact-v2 force and conservative residual assembly.
use crate::{provider::V2Force, v2_experiment::probes::ProbeFields, v2_run::Origin};
use nsbu_solver::{
    diagnostics::{conservative::ConservativeWorkspace, norms::Norms, residual::ResidualPlan},
    domain::{Domain, Layout},
    integrators::forcing::{ForceLimits, PrescribedForce},
    verification::reconstruction::OffStageProbe,
    Complex64, SolverError,
};

type Field = [Vec<Complex64>; 3];
struct Scratch {
    forcing: Field,
    conservative: Field,
    residual: Field,
    pressure: Vec<Complex64>,
}
impl Scratch {
    fn new(n: usize) -> Result<Self, SolverError> {
        Ok(Self {
            forcing: field(n)?,
            conservative: field(n)?,
            residual: field(n)?,
            pressure: filled(n)?,
        })
    }
}

/// Complete finite work for one residual workspace.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ResidualWork {
    /// Charged probes, including failed attempts.
    pub probes: usize,
    /// Fresh exact-v2 provider work units.
    pub provider_work_units: usize,
    /// Provider plus conservative-product scalar transforms.
    pub scalar_transforms: usize,
    /// Conservative coefficient and real-grid visits.
    pub coefficient_work_units: usize,
}

/// Fixed storage and work admission for one residual workspace.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ResidualBounds {
    /// Provider, fields and conservative scratch storage.
    pub storage_bytes: usize,
    /// Maximum charged probes.
    pub maximum_probes: usize,
    /// Complete admitted work.
    pub work: ResidualWork,
}

/// One full doubled-band residual norm with its actual reconstruction geometry.
#[derive(Debug, Clone, Copy)]
pub struct ResidualSample {
    geometry: OffStageProbe,
    origin: Origin,
    domain: Domain,
    diagnostic: Domain,
    force_samples: Layout,
    norms: Norms,
}
impl ResidualSample {
    /// Complete doubled-band residual norms at one exact probe.
    pub fn norms(self) -> Norms {
        self.norms
    }
    /// Actual accepted-node geometry.
    pub fn geometry(self) -> OffStageProbe {
        self.geometry
    }
    /// The internally constructed from-rest origin.
    pub fn origin(self) -> Origin {
        self.origin
    }
    /// Retained source domain; residual coefficients use its doubled domain.
    pub fn domain(self) -> Domain {
        self.domain
    }
    /// Child-specific doubled diagnostic domain.
    pub fn diagnostic_domain(self) -> Domain {
        self.diagnostic
    }
    /// Common exact-force sampling grid shared by all six children.
    pub fn force_sample_layout(self) -> Layout {
        self.force_samples
    }
}

pub(super) struct ResidualWorkspace {
    source: Domain,
    diagnostic: Domain,
    force_samples: Layout,
    bounds: ResidualBounds,
    force_limit: ForceLimits,
    force: V2Force,
    products: ConservativeWorkspace,
    forcing: Field,
    conservative: Field,
    residual: Field,
    pressure: Vec<Complex64>,
    work: ResidualWork,
}
impl ResidualWorkspace {
    pub(super) fn new(
        source: Domain,
        force_samples: Layout,
        maximum_probes: usize,
        cap: usize,
    ) -> Result<Self, SolverError> {
        let bounds = Self::reservation(source, force_samples, maximum_probes)?;
        if bounds.storage_bytes > cap {
            return Err(SolverError::ResourceLimit);
        }
        let diagnostic = ConservativeWorkspace::diagnostic_domain(source)?;
        let force_limit = V2Force::preflight(diagnostic, force_samples)?;
        let m = diagnostic.layout().half_len();
        let scratch = Scratch::new(m)?;
        Ok(Self {
            source,
            diagnostic,
            force_samples,
            bounds,
            force_limit,
            force: V2Force::new(diagnostic, force_samples, force_limit.storage_bytes)?,
            products: ConservativeWorkspace::new(
                source,
                ConservativeWorkspace::reservation(source)?,
            )?,
            forcing: scratch.forcing,
            conservative: scratch.conservative,
            residual: scratch.residual,
            pressure: scratch.pressure,
            work: ResidualWork::default(),
        })
    }
    pub(super) fn reservation(
        source: Domain,
        force_samples: Layout,
        maximum_probes: usize,
    ) -> Result<ResidualBounds, SolverError> {
        if maximum_probes == 0 {
            return Err(SolverError::ResourceLimit);
        }
        let diagnostic = ConservativeWorkspace::diagnostic_domain(source)?;
        let force = V2Force::preflight(diagnostic, force_samples)?;
        let storage_bytes = storage_reservation(source, diagnostic, force.storage_bytes)?;
        let per_probe = per_probe_work(diagnostic, force)?;
        Ok(ResidualBounds {
            storage_bytes,
            maximum_probes,
            work: ResidualWork {
                probes: maximum_probes,
                provider_work_units: mul(per_probe.provider_work_units, maximum_probes)?,
                scalar_transforms: mul(per_probe.scalar_transforms, maximum_probes)?,
                coefficient_work_units: mul(per_probe.coefficient_work_units, maximum_probes)?,
            },
        })
    }
    pub(super) fn consumption(&self) -> ResidualWork {
        self.work
    }
    pub(super) fn coefficients(&self) -> [&[Complex64]; 3] {
        self.residual.each_ref().map(Vec::as_slice)
    }
    pub(super) fn diagnostic_domain(&self) -> Domain {
        self.diagnostic
    }
    pub(super) fn measure(
        &mut self,
        fields: ProbeFields<'_>,
    ) -> Result<ResidualSample, SolverError> {
        self.charge()?;
        if fields.domain != self.source {
            return Err(SolverError::InvalidDomain);
        }
        let geometry = OffStageProbe::new(fields.origin.accepted_nodes, fields.clock)
            .map_err(|_| SolverError::InvalidClock)?;
        self.evaluate_force(fields.clock)?;
        let norms = self.assemble(fields)?;
        Ok(ResidualSample {
            geometry,
            origin: Origin::InternalFromRest,
            domain: self.source,
            diagnostic: self.diagnostic,
            force_samples: self.force_samples,
            norms,
        })
    }
    fn evaluate_force(&mut self, clock: nsbu_solver::domain::TickClock) -> Result<(), SolverError> {
        let spent = self.force.evaluate(
            clock,
            self.force_limit,
            self.forcing.each_mut().map(Vec::as_mut_slice),
        )?;
        if spent.work_units > self.force_limit.work_units
            || spent.scalar_transforms > self.force_limit.scalar_transforms
        {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        Ok(())
    }
    fn assemble(&mut self, fields: ProbeFields<'_>) -> Result<Norms, SolverError> {
        self.products.evaluate(
            fields.value,
            self.forcing.each_ref().map(Vec::as_slice),
            self.conservative.each_mut().map(Vec::as_mut_slice),
            &mut self.pressure,
        )?;
        ResidualPlan::new(self.source)?.evaluate(
            fields.value,
            fields.derivative,
            self.conservative.each_ref().map(Vec::as_slice),
            self.residual.each_mut().map(Vec::as_mut_slice),
        )
    }
    fn charge(&mut self) -> Result<(), SolverError> {
        if self.work.probes == self.bounds.maximum_probes {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        self.work.probes += 1;
        self.work.provider_work_units +=
            self.bounds.work.provider_work_units / self.bounds.maximum_probes;
        self.work.scalar_transforms +=
            self.bounds.work.scalar_transforms / self.bounds.maximum_probes;
        self.work.coefficient_work_units +=
            self.bounds.work.coefficient_work_units / self.bounds.maximum_probes;
        Ok(())
    }
}

fn storage_reservation(
    source: Domain,
    diagnostic: Domain,
    force_bytes: usize,
) -> Result<usize, SolverError> {
    let fields = mul(diagnostic.layout().half_len(), 10)?;
    add(
        add(
            add(
                mul(fields, std::mem::size_of::<Complex64>())?,
                ConservativeWorkspace::reservation(source)?,
            )?,
            force_bytes,
        )?,
        add(std::mem::size_of::<ResidualWorkspace>(), 11 * 64)?,
    )
}
fn per_probe_work(diagnostic: Domain, force: ForceLimits) -> Result<ResidualWork, SolverError> {
    let visits = add(
        add(
            mul(diagnostic.layout().half_len(), 128)?,
            mul(diagnostic.layout().real_len(), 6)?,
        )?,
        512,
    )?;
    Ok(ResidualWork {
        probes: 1,
        provider_work_units: force.work_units,
        scalar_transforms: add(force.scalar_transforms, 9)?,
        coefficient_work_units: visits,
    })
}

fn field(n: usize) -> Result<Field, SolverError> {
    Ok([filled(n)?, filled(n)?, filled(n)?])
}
fn filled(n: usize) -> Result<Vec<Complex64>, SolverError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(n)
        .map_err(|_| SolverError::AllocationFailed)?;
    values.resize(n, Complex64::new(0.0, 0.0));
    Ok(values)
}
fn add(a: usize, b: usize) -> Result<usize, SolverError> {
    a.checked_add(b).ok_or(SolverError::SizeOverflow)
}
fn mul(a: usize, b: usize) -> Result<usize, SolverError> {
    a.checked_mul(b).ok_or(SolverError::SizeOverflow)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::v2_experiment::probes::ProbeOrigin;
    use nsbu_solver::domain::TickClock;

    fn clock(elapsed: u128) -> TickClock {
        TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap()
    }

    #[test]
    fn zero_allowance_small_cap_wrong_domain_and_exhaustion_are_refused() {
        let source = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
        let samples = Layout::new([24; 3]).unwrap();
        assert!(ResidualWorkspace::reservation(source, samples, 0).is_err());
        let bounds = ResidualWorkspace::reservation(source, samples, 1).unwrap();
        assert!(ResidualWorkspace::new(source, samples, 1, bounds.storage_bytes - 1).is_err());

        let mut workspace =
            ResidualWorkspace::new(source, samples, 1, bounds.storage_bytes).unwrap();
        let wrong = Domain::new([8; 3], [1.0; 3], 1.0).unwrap();
        let empty: [&[Complex64]; 3] = [&[], &[], &[]];
        let fields = || ProbeFields {
            domain: wrong,
            clock: clock(7),
            origin: ProbeOrigin {
                accepted_nodes: [clock(0), clock(16), clock(32)],
                state_clock: clock(32),
            },
            value: empty,
            derivative: empty,
        };
        assert!(matches!(
            workspace.measure(fields()),
            Err(SolverError::InvalidDomain)
        ));
        assert!(matches!(
            workspace.measure(fields()),
            Err(SolverError::ProviderBudgetExceeded)
        ));
    }
}
