//! Dedicated-process allocation instrumentation across numerical accept/reject and swap commits.
use nsbu_solver::domain::{Domain, Epoch, ExtraStorage, ResourcePlan, SpectralState, TickClock};
use nsbu_solver::integrators::{
    attempt::AttemptWorkspace,
    forcing::{ForceLimits, ForceWork, PrescribedForce},
    indicator::Tolerances,
    method::Method,
    rhs::SpectralRhs,
    transaction::{commit_candidate, CandidateState},
};
use nsbu_solver::{Complex64, SolverError};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::alloc::System;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

struct OscillatingMean;
impl PrescribedForce for OscillatingMean {
    fn limits(&self) -> Option<ForceLimits> {
        Some(ForceLimits {
            storage_bytes: 0,
            work_units: 144,
            scalar_transforms: 0,
            remaining_divisor: 1,
        })
    }
    fn evaluate(
        &mut self,
        time: TickClock,
        _limit: ForceLimits,
        output: [&mut [Complex64]; 3],
    ) -> Result<ForceWork, SolverError> {
        for values in output {
            values.fill(Complex64::new(0.0, 0.0));
            values[0] = Complex64::new((time.elapsed() as f64).cos(), 0.0);
        }
        Ok(ForceWork {
            work_units: 144,
            scalar_transforms: 0,
        })
    }
}

fn main() {
    for (method, calls) in [(Method::CoxMatthews, 12), (Method::HochbruckOstermann, 15)] {
        probe(method, calls);
    }
}

fn probe(method: Method, calls: usize) {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let source =
        SpectralRhs::<OscillatingMean>::reservation(domain, OscillatingMean.limits().unwrap())
            .unwrap();
    let plan = ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: 0,
            force: source,
            diagnostics: AttemptWorkspace::reservation_with_method(domain, method).unwrap(),
            overhead: 4096,
        },
        2 * 1024 * 1024,
        Epoch(0),
    )
    .unwrap();
    let clock = TickClock::from_rest(-12, 4096).unwrap();
    let planning = Region::new(GLOBAL);
    let mut state = SpectralState::from_rest(plan, clock, Epoch(0)).unwrap();
    let mut candidate = CandidateState::new(plan, clock, Epoch(0)).unwrap();
    let mut work = AttemptWorkspace::new_with_method(plan, method).unwrap();
    let mut rhs = SpectralRhs::new(domain, OscillatingMean, 0.3, source).unwrap();
    assert!(planning.change().bytes_allocated <= plan.total());
    let original = state.component(0).unwrap().as_ptr();
    let region = Region::new(GLOBAL);
    for count in 1..=20 {
        let tight = Tolerances {
            absolute: [1e-30; 2],
            relative: [0.0; 2],
        };
        let rejected = work
            .try_advance(&state, &mut candidate, 4, tight, &mut rhs)
            .unwrap();
        assert!(rejected.accepted.is_none());
        let loose = Tolerances {
            absolute: [1.0; 2],
            relative: [0.0; 2],
        };
        let stale = work
            .try_advance(&state, &mut candidate, 4, loose, &mut rhs)
            .unwrap()
            .accepted
            .unwrap();
        let accepted = work
            .try_advance(&state, &mut candidate, 4, loose, &mut rhs)
            .unwrap()
            .accepted
            .unwrap();
        assert_eq!(
            commit_candidate(plan, &mut state, &mut candidate, stale),
            Err(SolverError::StaleAttempt)
        );
        commit_candidate(plan, &mut state, &mut candidate, accepted).unwrap();
        assert_eq!(state.accepted_steps(), count);
        assert_eq!(rhs.consumption(), [calls, calls * 144, calls * 10]);
    }
    assert_eq!(state.component(0).unwrap().as_ptr(), original);
    let stats = region.change();
    assert_eq!(
        (stats.allocations, stats.deallocations, stats.reallocations),
        (0, 0, 0)
    );
    assert_eq!((stats.bytes_allocated, stats.bytes_deallocated), (0, 0));
}
