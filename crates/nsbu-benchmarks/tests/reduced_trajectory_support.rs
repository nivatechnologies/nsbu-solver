//! Joint admission and separate numerical owners for the optional-provider trajectory tests.
use nsbu_benchmarks::provider::{reduced::ReducedV2Force, V2Force};
use nsbu_solver::{
    domain::{Domain, Epoch, ExtraStorage, ResourcePlan, SpectralState, TickClock},
    integrators::{
        attempt::AttemptWorkspace, forcing::ForceLimits, method::Method, rhs::SpectralRhs,
    },
};

/// Fixed N4 unit cube with the exact-v2 viscosity.
pub fn domain() -> Domain {
    Domain::new([4; 3], [1.0; 3], 1.0).unwrap()
}
/// Exact rest clock with quantum 2^-20 and target time 1/128.
pub fn clock() -> TickClock {
    TickClock::restore(-20, 8192, 0, 8192).unwrap()
}
/// Fixed step in exact ticks; the retained two-half candidate is committed.
pub const STEP: u128 = 128;
/// First endpoint 1/256 expressed in the fixed clock ticks.
pub const ENDPOINT: u128 = 4096;
/// Complete paired-owner and test metadata storage cap in bytes.
pub const CAP: usize = 8 * 1024 * 1024;
/// Finite maximum attempt count per owner in this fixed profile.
pub const STEPS: usize = 32;

/// Independently allocated numerical owner, with no analytical assignment interface.
pub struct Trajectory<F: nsbu_solver::integrators::forcing::PrescribedForce> {
    /// Committed state, initially exact rest.
    pub state: SpectralState,
    /// Separate candidate storage; only a valid acceptance token can commit it.
    pub candidate: nsbu_solver::integrators::transaction::CandidateState,
    /// Preallocated full/two-half attempt scratch for the selected method.
    pub workspace: AttemptWorkspace,
    /// Independently owned prescribed provider and spectral RHS scratch.
    pub rhs: SpectralRhs<F>,
    /// Total provider work-unit allowance across the fixed attempt count.
    pub maximum_work: usize,
}

struct Prepared {
    domain: Domain,
    limits: ForceLimits,
    rhs_bytes: usize,
    plan: ResourcePlan,
    maximum_work: usize,
}

fn prepare<F: nsbu_solver::integrators::forcing::PrescribedForce>(
    method: Method,
    preflight: fn(
        Domain,
        nsbu_solver::domain::Layout,
    ) -> Result<ForceLimits, nsbu_solver::SolverError>,
) -> Prepared {
    let domain = domain();
    let limits = preflight(domain, domain.layout()).unwrap();
    let rhs_bytes = SpectralRhs::<F>::reservation(domain, limits).unwrap();
    let diagnostics = AttemptWorkspace::reservation_with_method(domain, method).unwrap();
    let plan = ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: 0,
            force: rhs_bytes,
            diagnostics,
            overhead: 1024 * 1024,
        },
        CAP / 2,
        Epoch(0),
    )
    .unwrap();
    let per_attempt = limits.work_units.checked_mul(method.rhs_calls()).unwrap();
    let maximum_work = per_attempt.checked_mul(STEPS).unwrap();
    assert!(limits.storage_bytes > 0);
    Prepared {
        domain,
        limits,
        rhs_bytes,
        plan,
        maximum_work,
    }
}

/// Admit both complete owners and finite work before constructing either provider.
/// The return order is original Cartesian, then optional reduced arithmetic.
pub fn make_pair(method: Method) -> (Trajectory<V2Force>, Trajectory<ReducedV2Force>) {
    // Declarations, checked sums and cap−1 refusals precede either provider allocation.
    let original = prepare::<V2Force>(method, V2Force::preflight);
    let reduced = prepare::<ReducedV2Force>(method, ReducedV2Force::preflight);
    assert!(
        original
            .plan
            .total()
            .checked_add(reduced.plan.total())
            .unwrap()
            <= CAP
    );
    assert!(original
        .maximum_work
        .checked_add(reduced.maximum_work)
        .is_some());
    assert!(V2Force::new(
        original.domain,
        original.domain.layout(),
        original.limits.storage_bytes - 1
    )
    .is_err());
    assert!(ReducedV2Force::new(
        reduced.domain,
        reduced.domain.layout(),
        reduced.limits.storage_bytes - 1
    )
    .is_err());
    let original_provider = V2Force::new(
        original.domain,
        original.domain.layout(),
        original.limits.storage_bytes,
    )
    .unwrap();
    let reduced_provider = ReducedV2Force::new(
        reduced.domain,
        reduced.domain.layout(),
        reduced.limits.storage_bytes,
    )
    .unwrap();
    (
        make(original, method, original_provider),
        make(reduced, method, reduced_provider),
    )
}

fn make<F: nsbu_solver::integrators::forcing::PrescribedForce>(
    prepared: Prepared,
    method: Method,
    provider: F,
) -> Trajectory<F> {
    let clock = clock();
    let state = SpectralState::from_rest(prepared.plan, clock, Epoch(0)).unwrap();
    let candidate =
        nsbu_solver::integrators::transaction::CandidateState::new(prepared.plan, clock, Epoch(0))
            .unwrap();
    let workspace = AttemptWorkspace::new_with_method(prepared.plan, method).unwrap();
    let rhs = SpectralRhs::new(prepared.domain, provider, 0.3, prepared.rhs_bytes).unwrap();
    Trajectory {
        state,
        candidate,
        workspace,
        rhs,
        maximum_work: prepared.maximum_work,
    }
}

/// Snapshot every retained real/imaginary coefficient word without rounding.
pub fn bits(state: &SpectralState) -> Vec<(u64, u64)> {
    (0..3)
        .flat_map(|axis| state.component(axis).unwrap())
        .map(|v| (v.re.to_bits(), v.im.to_bits()))
        .collect()
}
