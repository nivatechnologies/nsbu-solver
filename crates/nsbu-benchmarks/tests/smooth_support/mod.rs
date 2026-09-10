//! Shared bounded from-rest execution for smooth temporal and diagnostic experiments.
use nsbu_benchmarks::smooth::CyclicSine;
use nsbu_solver::{
    domain::{Domain, Epoch, ExtraStorage, ResourcePlan, SpectralState, TickClock},
    integrators::{
        attempt::AttemptWorkspace,
        forcing::PrescribedForce,
        indicator::Tolerances,
        method::Method,
        rhs::SpectralRhs,
        trajectory::{FixedRun, RunLimits, StopReason},
        transaction::CandidateState,
    },
};

pub struct SmoothRun {
    pub state: SpectralState,
    candidate: CandidateState,
    attempts: AttemptWorkspace,
    rhs: SpectralRhs<CyclicSine>,
}
impl SmoothRun {
    pub fn new(domain: Domain, method: Method, diagnostics: usize) -> Self {
        let source = CyclicSine::new(domain).unwrap();
        let reservation =
            SpectralRhs::<CyclicSine>::reservation(domain, source.limits().unwrap()).unwrap();
        let plan = ResourcePlan::new(
            domain,
            ExtraStorage {
                fft: 0,
                force: reservation,
                diagnostics: AttemptWorkspace::reservation_with_method(domain, method)
                    .unwrap()
                    .checked_add(diagnostics)
                    .unwrap(),
                overhead: 1024 * 1024,
            },
            8 * 1024 * 1024,
            Epoch(0),
        )
        .unwrap();
        let clock = TickClock::from_rest(-20, 1 << 20).unwrap();
        Self {
            state: SpectralState::from_rest(plan, clock, Epoch(0)).unwrap(),
            candidate: CandidateState::new(plan, clock, Epoch(0)).unwrap(),
            attempts: AttemptWorkspace::new_with_method(plan, method).unwrap(),
            rhs: SpectralRhs::new(domain, source, 0.3, reservation).unwrap(),
        }
    }
    pub fn advance(&mut self, endpoint: u128, step: u128) {
        let count = ((endpoint - self.state.clock().elapsed()) / step) as usize;
        let report = FixedRun::new(
            &mut self.state,
            &mut self.candidate,
            &mut self.attempts,
            &mut self.rhs,
        )
        .execute(
            RunLimits {
                endpoint,
                step_ticks: step,
                maximum_attempts: count,
            },
            Tolerances {
                absolute: [1e-2; 2],
                relative: [0.0; 2],
            },
        )
        .unwrap();
        assert_eq!(report.reason, StopReason::EndpointReached);
        assert_eq!(report.committed, count);
        assert_eq!(report.clock.elapsed(), endpoint);
    }
}
