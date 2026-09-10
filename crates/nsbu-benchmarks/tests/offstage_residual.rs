//! Residuals use accepted history, independent products and exact off-stage force times.
mod owned_reconstruction_support;
use nsbu_benchmarks::{
    smooth_experiment::residual::ResidualWorkspace,
    smooth_run::{reconstructed_archive, Origin, ReconstructedRun},
};
use nsbu_solver::{
    domain::{Domain, TickClock},
    experiment::control::Outcome,
    integrators::method::Method,
    SolverError,
};
use owned_reconstruction_support::{configuration, run, CAP};
fn probe(tick: u128) -> TickClock {
    TickClock::restore(-20, 1 << 20, tick, (1 << 20) - tick).unwrap()
}
#[test]
fn independent_offstage_residual_refines_without_changing_accepted_states() {
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        let mut previous = f64::INFINITY;
        for step in [1024, 512, 256] {
            let mut config = configuration(method, 1e-2);
            config.limits.endpoint = 2048;
            config.limits.step_ticks = step;
            config.limits.maximum_attempts = (2048 / step) as usize;
            let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
            let mut trajectory = ReconstructedRun::from_rest(
                domain,
                probe(0),
                config,
                config.limits.maximum_attempts + 1,
                0.3,
                CAP,
            )
            .unwrap();
            for _ in 0..config.limits.maximum_attempts {
                assert!(matches!(trajectory.step().unwrap(), Outcome::Committed(_)));
            }
            let before = (
                trajectory.state().clock(),
                trajectory.state().epoch(),
                trajectory.observer_work(),
            );
            let physical = trajectory.state().component(0).unwrap().to_vec();
            let mut diagnostic = ResidualWorkspace::new(domain, 1, CAP).unwrap();
            let result = diagnostic.measure(&trajectory, probe(2047)).unwrap();
            assert_eq!(result.origin(), Origin::InternalFromRest);
            assert_eq!(result.domain(), domain);
            assert_eq!(result.geometry().time(), probe(2047));
            let norms = result.norms();
            eprintln!("{method:?} step={step} residual={norms:?}");
            assert!(norms.h1 < previous * 0.5);
            assert!(norms.h1 < 1e-4);
            assert!(norms.l2 <= norms.h1);
            assert!(norms.divergence_l2 < 1e-10);
            previous = norms.h1;
            assert_eq!(
                before,
                (
                    trajectory.state().clock(),
                    trajectory.state().epoch(),
                    trajectory.observer_work()
                )
            );
            assert_eq!(physical, trajectory.state().component(0).unwrap());
            assert_eq!(diagnostic.consumption(), diagnostic.bounds().work);
            assert_eq!(diagnostic.remaining(), 0);
            assert!(diagnostic.measure(&trajectory, probe(2047)).is_err());
        }
    }
}
#[test]
fn missing_history_stage_probes_and_external_origin_keep_their_restrictions() {
    let mut trajectory = run(Method::CoxMatthews, 1e-2, 5);
    let domain = trajectory.state().plan().domain();
    let mut diagnostic = ResidualWorkspace::new(domain, 4, CAP).unwrap();
    assert!(diagnostic.measure(&trajectory, probe(769)).is_err());
    trajectory.step().unwrap();
    trajectory.step().unwrap();
    for t in [0, 768, 2049] {
        assert!(diagnostic.measure(&trajectory, probe(t)).is_err());
    }
    assert_eq!(diagnostic.consumption(), diagnostic.bounds().work);
    let mut bytes = vec![0; reconstructed_archive::encoded_len(&trajectory).unwrap()];
    reconstructed_archive::write(&trajectory, &mut bytes).unwrap();
    let imported = reconstructed_archive::read(&bytes, trajectory.state().plan(), bytes.len(), CAP)
        .unwrap()
        .continue_unverified(CAP)
        .unwrap();
    let result = ResidualWorkspace::new(domain, 1, CAP)
        .unwrap()
        .measure(&imported, probe(769))
        .unwrap();
    assert_eq!(result.origin(), Origin::ExternalUnverified);
}
#[test]
fn admission_and_wrong_domain_fail_with_bounded_charges() {
    let trajectory = run(Method::CoxMatthews, 1e-2, 5);
    let domain = trajectory.state().plan().domain();
    assert!(ResidualWorkspace::reservation(domain, 0).is_err());
    assert!(ResidualWorkspace::reservation(domain, usize::MAX).is_err());
    assert!(matches!(
        ResidualWorkspace::new(domain, 1, 1),
        Err(SolverError::ResourceLimit)
    ));
    let wrong = Domain::new([8; 3], [1.0; 3], 1.0).unwrap();
    let mut diagnostic = ResidualWorkspace::new(wrong, 1, CAP).unwrap();
    assert!(diagnostic.measure(&trajectory, probe(769)).is_err());
    assert_eq!(diagnostic.consumption(), diagnostic.bounds().work);
}
