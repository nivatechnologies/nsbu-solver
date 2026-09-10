//! Actual independently evolved smooth CM/HO fields feed complete physical comparisons.
mod owned_reconstruction_support;
use nsbu_benchmarks::smooth_run::{ReconstructedPlan, ReconstructedRun};
use nsbu_solver::{
    diagnostics::physical::{PhysicalComparisonWorkspace, PhysicalField, PhysicalQuantity},
    domain::{Domain, Layout, TickClock},
    experiment::control::Outcome,
    integrators::method::Method,
};
use sha2::{Digest, Sha256};

fn state_digest(run: &ReconstructedRun) -> [u8; 32] {
    let mut hash = Sha256::new();
    for component in 0..3 {
        for value in run.state().component(component).unwrap() {
            hash.update(value.re.to_bits().to_le_bytes());
            hash.update(value.im.to_bits().to_le_bytes());
        }
    }
    hash.finalize().into()
}
fn view(run: &ReconstructedRun) -> PhysicalField<'_> {
    PhysicalField::Vector(std::array::from_fn(|axis| {
        run.state().component(axis).unwrap()
    }))
}

#[test]
fn complete_method_comparisons_preserve_both_actual_from_rest_states() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let samples = Layout::new([8; 3]).unwrap();
    let clock = TickClock::from_rest(-20, 1 << 20).unwrap();
    let diagnostic = PhysicalComparisonWorkspace::reservation(domain, domain, samples).unwrap();
    let plans = [Method::CoxMatthews, Method::HochbruckOstermann].map(|method| {
        ReconstructedPlan::from_rest(
            domain,
            clock,
            owned_reconstruction_support::configuration(method, 1e-2),
            5,
            0.3,
            owned_reconstruction_support::CAP,
        )
        .unwrap()
    });
    let total = plans
        .iter()
        .try_fold(diagnostic, |bytes, plan| {
            bytes.checked_add(plan.resources().total())
        })
        .unwrap();
    assert!(total < 16 * 1024 * 1024);
    let [mut left, mut right] = [Method::CoxMatthews, Method::HochbruckOstermann]
        .map(|method| owned_reconstruction_support::run(method, 1e-2, 5));
    let mut workspace =
        PhysicalComparisonWorkspace::new(domain, domain, samples, diagnostic).unwrap();
    for run in [&mut left, &mut right] {
        for _ in 0..4 {
            assert!(matches!(run.step().unwrap(), Outcome::Committed(_)));
        }
        assert_eq!(run.history().controller().committed(), 4);
    }
    assert_eq!(left.state().clock(), right.state().clock());
    let before = [state_digest(&left), state_digest(&right)];
    for quantity in [
        PhysicalQuantity::Vector,
        PhysicalQuantity::Gradient,
        PhysicalQuantity::Hessian,
        PhysicalQuantity::Vorticity,
    ] {
        let result = workspace
            .compare(view(&left), view(&right), quantity, 1e-8)
            .unwrap();
        let error = result.global();
        assert!(error.rms_error > 0.0 && error.rms_error < 1e-6);
        assert!(error.peak_error >= error.rms_error);
        println!(
            "actual CM/HO {quantity:?}: RMS={} peak={}; accepted_pde_windows=0",
            error.rms_error, error.peak_error
        );
    }
    assert_eq!([state_digest(&left), state_digest(&right)], before);
}
