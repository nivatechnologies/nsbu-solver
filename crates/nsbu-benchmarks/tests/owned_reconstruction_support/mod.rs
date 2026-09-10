//! Bounded owned reconstruction profile shared by numerical and allocation contracts.
use nsbu_benchmarks::smooth_run::ReconstructedRun;
use nsbu_solver::{
    domain::{Domain, TickClock},
    experiment::control::Configuration,
    integrators::{indicator::Tolerances, method::Method, trajectory::RunLimits},
    Complex64,
};
pub const CAP: usize = 8 * 1024 * 1024;
pub fn configuration(method: Method, tolerance: f64) -> Configuration {
    Configuration {
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
    }
}
pub fn run(method: Method, tolerance: f64, samples: usize) -> ReconstructedRun {
    ReconstructedRun::from_rest(
        Domain::new([4; 3], [1.0; 3], 1.0).unwrap(),
        TickClock::from_rest(-20, 1 << 20).unwrap(),
        configuration(method, tolerance),
        samples,
        0.3,
        CAP,
    )
    .unwrap()
}

pub fn compare(left: &ReconstructedRun, right: &ReconstructedRun) {
    assert_eq!(left.state().clock(), right.state().clock());
    assert_eq!(left.state().epoch(), right.state().epoch());
    assert_eq!(left.work(), right.work());
    assert_eq!(left.observer_work(), right.observer_work());
    assert_eq!(
        left.observer().modal_visits(),
        right.observer().modal_visits()
    );
    assert_eq!(
        left.observer().remaining_samples(),
        right.observer().remaining_samples()
    );
    assert_eq!(
        left.history().controller().stopped(),
        right.history().controller().stopped()
    );
    assert_eq!(
        left.history().records().len(),
        right.history().records().len()
    );
    for (a, b) in left
        .history()
        .records()
        .iter()
        .zip(right.history().records())
    {
        assert_eq!(
            (a.start, a.outcome, a.sample),
            (b.start, b.outcome, b.sample)
        );
    }
    for axis in 0..3 {
        assert_bits(
            left.state().component(axis).unwrap(),
            right.state().component(axis).unwrap(),
        );
    }
    compare_reconstruction(left, right);
}
fn assert_bits(left: &[Complex64], right: &[Complex64]) {
    assert_eq!(left.len(), right.len());
    for (a, b) in left.iter().zip(right) {
        assert_eq!(
            [a.re.to_bits(), a.im.to_bits()],
            [b.re.to_bits(), b.im.to_bits()]
        );
    }
}
fn compare_reconstruction(left: &ReconstructedRun, right: &ReconstructedRun) {
    let clocks = left.observer().last_accepted_clocks();
    assert_eq!(clocks, right.observer().last_accepted_clocks());
    let Some(clocks) = clocks else {
        return;
    };
    let elapsed = clocks[1].elapsed() + 1;
    let probe = TickClock::restore(
        clocks[0].exponent(),
        clocks[0].target(),
        elapsed,
        clocks[0].target() - elapsed,
    )
    .unwrap();
    let zero = vec![Complex64::new(0.0, 0.0); left.state().plan().domain().layout().half_len()];
    let [mut a, mut b, mut da, mut db] = [zero.clone(), zero.clone(), zero.clone(), zero];
    for axis in 0..3 {
        left.observer()
            .reconstruct(probe, axis, &mut a, &mut da)
            .unwrap();
        right
            .observer()
            .reconstruct(probe, axis, &mut b, &mut db)
            .unwrap();
        assert_bits(&a, &b);
        assert_bits(&da, &db);
    }
}
