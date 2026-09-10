//! Complete coarse exact-v2 trajectory from rest, compared with independent direct-DFT evolution.
mod fixture_support;
use nsbu_benchmarks::provider::V2Force;
use nsbu_solver::{
    domain::{Domain, Epoch, ExtraStorage, ResourcePlan, SpectralState, TickClock},
    integrators::{
        attempt::AttemptWorkspace,
        indicator::Tolerances,
        rhs::SpectralRhs,
        trajectory::{FixedRun, RunLimits, StopReason},
        transaction::CandidateState,
    },
};

#[test]
fn concentrating_transactional_trajectory_matches_independent_dft() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let limits = V2Force::preflight(domain, domain.layout()).unwrap();
    let rhs_bytes = SpectralRhs::<V2Force>::reservation(domain, limits).unwrap();
    let cap = 2 * 1024 * 1024;
    let plan = ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: 0,
            force: rhs_bytes,
            diagnostics: AttemptWorkspace::reservation(domain).unwrap(),
            overhead: 1024 * 1024,
        },
        cap,
        Epoch(0),
    )
    .unwrap();
    let clock = TickClock::from_rest(-20, 8192).unwrap();
    let mut state = SpectralState::from_rest(plan, clock, Epoch(0)).unwrap();
    let mut candidate = CandidateState::new(plan, clock, Epoch(0)).unwrap();
    let mut workspace = AttemptWorkspace::new(plan).unwrap();
    let provider = V2Force::new(domain, domain.layout(), limits.storage_bytes).unwrap();
    let mut rhs = SpectralRhs::new(domain, provider, 0.3, rhs_bytes).unwrap();
    let report = FixedRun::new(&mut state, &mut candidate, &mut workspace, &mut rhs)
        .execute(
            RunLimits {
                endpoint: 4096,
                step_ticks: 128,
                maximum_attempts: 32,
            },
            Tolerances {
                absolute: [1e-5, 1e-4],
                relative: [1e-5; 2],
            },
        )
        .unwrap();
    assert_eq!(report.reason, StopReason::EndpointReached);
    assert_eq!(report.clock.remaining(), 4096);
    assert_eq!(report.committed, 32);
    assert_eq!(report.attempted, 32);
    assert_eq!(state.accepted_steps(), 32);
    assert_eq!(rhs.consumption()[0], 12);
    assert_eq!(rhs.consumption()[2], 156);
    let output = std::array::from_fn(|axis| state.component(axis).unwrap().to_vec());
    fixture_support::compare(
        domain.layout(),
        &output,
        include_str!("fixtures/concentrating-n4.tsv"),
        5e-13,
    );
    let reservation = plan.total();
    let ratios = report.maximum_local_ratios;
    println!("concentrating diagnostic-only n=4 force_grid=4 endpoint=1/256 macro_steps=32 fine_cm_steps=64 rhs_calls=384 transforms=4992 reference_assignments=0 accepted_pde_windows=0 reserved_bytes={reservation} cap_bytes={cap} max_local_ratios={ratios:?}");
}
