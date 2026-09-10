//! Actual accepted-state transactions feed independent off-stage reconstruction.
use nsbu_benchmarks::{
    smooth::CyclicSine, smooth_observer::reconstruction::ReconstructionObserver,
};
use nsbu_solver::{
    domain::{Domain, Epoch, ExtraStorage, ResourcePlan, SpectralState, TickClock},
    experiment::{
        control::{Configuration, Outcome},
        log::RunHistory,
        runner::recorded_step,
    },
    integrators::{
        attempt::AttemptWorkspace, forcing::PrescribedForce, indicator::Tolerances, method::Method,
        rhs::SpectralRhs, trajectory::RunLimits, transaction::CandidateState,
    },
};

pub struct Case {
    pub state: SpectralState,
    candidate: CandidateState,
    workspace: AttemptWorkspace,
    rhs: SpectralRhs<CyclicSine>,
    pub observer: ReconstructionObserver,
    history: RunHistory,
}
impl Case {
    pub fn new(method: Method, samples: usize, tolerance: f64) -> Self {
        let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
        let clock = TickClock::from_rest(-20, 1 << 20).unwrap();
        let configuration = Configuration {
            method,
            limits: RunLimits {
                endpoint: 4096,
                step_ticks: 1024,
                maximum_attempts: 4,
            },
            tolerances: Tolerances {
                absolute: [tolerance; 2],
                relative: [0.0; 2],
            },
        };
        let provider = CyclicSine::new(domain).unwrap();
        let force =
            SpectralRhs::<CyclicSine>::reservation(domain, provider.limits().unwrap()).unwrap();
        let diagnostic = ReconstructionObserver::limits(domain, samples).unwrap();
        let history_bytes = RunHistory::reservation(configuration).unwrap();
        let plan = ResourcePlan::new(
            domain,
            ExtraStorage {
                fft: 0,
                force,
                diagnostics: diagnostic.storage_bytes
                    + AttemptWorkspace::reservation_with_method(domain, method).unwrap(),
                overhead: history_bytes + 4096,
            },
            8 * 1024 * 1024,
            Epoch(0),
        )
        .unwrap();
        let state = SpectralState::from_rest(plan, clock, Epoch(0)).unwrap();
        let observer = ReconstructionObserver::new(plan, samples, &state).unwrap();
        Self {
            state,
            candidate: CandidateState::new(plan, clock, Epoch(0)).unwrap(),
            workspace: AttemptWorkspace::new_with_method(plan, method).unwrap(),
            rhs: SpectralRhs::new(domain, provider, 0.3, force).unwrap(),
            observer,
            history: RunHistory::new(clock, configuration, history_bytes).unwrap(),
        }
    }
    pub fn step(&mut self) -> Outcome {
        recorded_step(
            &mut self.state,
            &mut self.candidate,
            &mut self.workspace,
            &mut self.rhs,
            &mut self.observer,
            &mut self.history,
        )
        .unwrap()
    }
}
