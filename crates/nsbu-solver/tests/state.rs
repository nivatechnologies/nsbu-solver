//! Trajectories own disjoint zeroed buffers after an approved storage preflight.
use nsbu_solver::domain::{Domain, Epoch, ExtraStorage, ResourcePlan, SpectralState, TickClock};
use nsbu_solver::{Complex64, SolverError};

#[test]
fn states_own_separate_buffers_and_preserve_their_exact_initial_metadata() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let extra = ExtraStorage {
        fft: 0,
        force: 0,
        diagnostics: 0,
        overhead: 4096,
    };
    let plan = ResourcePlan::new(domain, extra, 1024 * 1024, Epoch(7)).unwrap();
    let clock = TickClock::from_rest(-20, 8192).unwrap();
    let first = SpectralState::from_rest(plan, clock, Epoch(2)).unwrap();
    let second = SpectralState::from_rest(plan, clock, Epoch(3)).unwrap();
    assert_eq!(first.plan(), plan);
    assert_eq!(first.clock(), clock);
    assert_eq!(first.epoch(), Epoch(2));
    assert_eq!(second.epoch(), Epoch(3));
    let mut pointers = Vec::new();
    for state in [&first, &second] {
        for axis in 0..3 {
            let component = state.component(axis).unwrap();
            assert_eq!(component.len(), 48);
            assert!(component
                .iter()
                .all(|&value| value == Complex64::new(0.0, 0.0)));
            assert!(!pointers.contains(&component.as_ptr()));
            pointers.push(component.as_ptr());
        }
    }
    assert_eq!(first.component(3), Err(SolverError::InvalidIndex));
    assert_eq!(first.component(usize::MAX), Err(SolverError::InvalidIndex));
    let late = clock.stages(4).unwrap()[4];
    assert!(matches!(
        SpectralState::from_rest(plan, late, Epoch(0)),
        Err(SolverError::InvalidClock)
    ));
}
