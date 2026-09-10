//! A nodal residual cannot replace off-stage reconstruction evidence.
use nsbu_solver::{
    domain::TickClock,
    verification::{reconstruction::OffStageProbe, VerificationError},
    SolverError,
};

fn clock(elapsed: u128) -> TickClock {
    TickClock::restore(-80, 1 << 70, elapsed, (1 << 70) - elapsed).unwrap()
}
fn probe(nodes: [u128; 3], time: u128) -> OffStageProbe {
    OffStageProbe::new(nodes.map(clock), clock(time)).unwrap()
}

#[test]
fn common_probes_survive_nested_refinement_without_losing_large_exact_ticks() {
    let base = 1 << 60;
    let nodes = [base, base + 16, base + 32];
    let coarse = probe(nodes, base + 3);
    let fine = probe([base, base + 8, base + 16], base + 3);
    assert_eq!(coarse.nodes(), nodes.map(clock));
    assert_eq!(fine.time(), clock(base + 3));
    assert!(fine.refines(coarse));
    assert!(!coarse.refines(fine));
    assert!(!coarse.refines(coarse));
    let moved = probe([base, base + 8, base + 16], base + 5);
    assert!(!moved.refines(coarse));
    let before = probe([base - 4, base + 4, base + 12], base + 3);
    assert!(!before.refines(coarse));
    let later_coarse = probe(nodes, base + 27);
    let after = probe([base + 24, base + 32, base + 40], base + 27);
    assert!(!after.refines(later_coarse));
}

#[test]
fn every_full_and_half_step_stage_is_refused_as_off_stage_evidence() {
    let nodes = [0, 16, 32].map(clock);
    for elapsed in (0..=32).step_by(4) {
        assert_eq!(
            OffStageProbe::new(nodes, clock(elapsed)).unwrap_err(),
            VerificationError::InvalidProbe
        );
    }
    for (nodes, time) in [([0, 6, 12], 1), ([0, 4, 8], 1)] {
        assert_eq!(
            OffStageProbe::new(nodes.map(clock), clock(time)).unwrap_err(),
            VerificationError::InvalidProbe
        );
    }
}

#[test]
fn invalid_history_and_unrepresentable_reconstruction_remain_explicit() {
    for (nodes, time) in [
        ([0, 8, 20], 3),
        ([8, 16, 24], 3),
        ([0, 8, 16], 17),
        ([0, 0, 0], 0),
    ] {
        assert_eq!(
            OffStageProbe::new(nodes.map(clock), clock(time)).unwrap_err(),
            VerificationError::ReconstructionGeometry(SolverError::InvalidClock)
        );
    }
    let tiny =
        [0, 8, 16].map(|elapsed| TickClock::restore(-2000, 32, elapsed, 32 - elapsed).unwrap());
    let time = TickClock::restore(-2000, 32, 3, 29).unwrap();
    assert_eq!(
        OffStageProbe::new(tiny, time).unwrap_err(),
        VerificationError::ReconstructionGeometry(SolverError::ArithmeticResolutionLimited)
    );
}
