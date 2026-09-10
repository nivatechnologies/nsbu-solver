//! Complete physical probes bind the physical time and preserve all accepted states/interpolants.
mod smooth_family_support;
use nsbu_benchmarks::smooth_experiment::{
    probes::{
        diagnostics::{ProbeDiagnostics, ProbeDiagnosticsPlan},
        ProbeFamily, ProbePlan,
    },
    FamilyError, FamilyPlan,
};
use nsbu_solver::{
    diagnostics::physical::{PhysicalComparisonWorkspace, PhysicalField, PhysicalQuantity},
    domain::{Layout, TickClock},
    verification::times::TestedTimes,
};
use sha2::{Digest, Sha256};
const FLOORS: [f64; 6] = [1e-8, 1e-7, 1e-6, 1e-7, 1e-8, 1e-7];
fn clocks() -> [TickClock; 3] {
    [0, 7, 128].map(|t| TickClock::restore(-16, 512, t, 512 - t).unwrap())
}
fn plan<'a>(accepted: &'a [TickClock], probes: &'a [TickClock], tolerance: f64) -> ProbePlan<'a> {
    let family = FamilyPlan::new(
        smooth_family_support::settings(tolerance),
        TestedTimes::new(accepted, accepted.len()).unwrap(),
        smooth_family_support::CAP,
    )
    .unwrap();
    ProbePlan::new(
        family,
        TestedTimes::new(probes, probes.len()).unwrap(),
        probes.len(),
        smooth_family_support::CAP,
    )
    .unwrap()
}
fn diagnostic<'a>(plan: ProbePlan<'a>, attempts: usize) -> ProbeDiagnosticsPlan<'a> {
    ProbeDiagnosticsPlan::new(
        plan,
        Layout::new([24; 3]).unwrap(),
        FLOORS,
        attempts,
        smooth_family_support::CAP,
    )
    .unwrap()
}
fn digest(family: &ProbeFamily<'_>) -> [u8; 32] {
    let mut hash = Sha256::new();
    for index in 0..6 {
        let view = family.fields(index).unwrap();
        let state = family.branch(index).unwrap().state();
        hash.update(state.clock().elapsed().to_le_bytes());
        for axis in 0..3 {
            for values in [
                state.component(axis).unwrap(),
                view.value[axis],
                view.derivative[axis],
            ] {
                for value in values {
                    hash.update(value.re.to_bits().to_le_bytes());
                    hash.update(value.im.to_bits().to_le_bytes());
                }
            }
        }
    }
    hash.finalize().into()
}

#[test]
fn complete_physical_probes_use_probe_time_and_leave_every_owned_field_unchanged() {
    let accepted = smooth_family_support::clocks();
    let times = clocks();
    let plan = plan(&accepted, &times, 1e-2);
    let admission = diagnostic(plan, 3);
    assert_eq!(admission.probe_plan().tested_times().as_slice(), &times);
    let mut family = ProbeFamily::new(plan).unwrap();
    let mut consumer = ProbeDiagnostics::new(admission).unwrap();
    for clock in times {
        let reconstructed = family.advance().unwrap().unwrap();
        let before = digest(&family);
        assert_eq!(consumer.next_time(), Some(clock));
        let measured = consumer.measure(&family).unwrap();
        assert_eq!(measured.clock(), clock);
        assert_eq!(measured.sample_layout(), admission.sample_layout());
        assert_eq!(measured.reconstruction().origins(), reconstructed.origins());
        assert_eq!(digest(&family), before);
        for quantity in measured.quantities() {
            for error in quantity
                .space
                .into_iter()
                .chain(quantity.time)
                .chain([quantity.method])
            {
                assert!(error.rms_error.is_finite());
                assert!(error.reference_peak.is_finite());
                if clock.elapsed() == 0 {
                    assert_eq!(error.rms_error, 0.0);
                    assert_eq!(error.reference_peak, 0.0);
                }
            }
        }
        if clock.elapsed() == 7 {
            let value = &measured.quantities()[0];
            assert!(value.time[1].rms_error > 0.0);
            assert!(measured
                .reconstruction()
                .origins()
                .iter()
                .all(|o| o.state_clock.elapsed() > 7));
            // Directly sample the later lookahead states: these are a different physical time
            // and must not have been silently substituted into the reconstructed comparison.
            let left = family.branch(3).unwrap().state();
            let right = family.branch(4).unwrap().state();
            let fields = |state: &nsbu_solver::domain::SpectralState| {
                [
                    state.component(0).unwrap().to_vec(),
                    state.component(1).unwrap().to_vec(),
                    state.component(2).unwrap().to_vec(),
                ]
            };
            let a = fields(left);
            let b = fields(right);
            let mut direct = PhysicalComparisonWorkspace::new(
                left.plan().domain(),
                right.plan().domain(),
                admission.sample_layout(),
                smooth_family_support::CAP,
            )
            .unwrap();
            let later = direct
                .compare(
                    PhysicalField::Vector(a.each_ref().map(Vec::as_slice)),
                    PhysicalField::Vector(b.each_ref().map(Vec::as_slice)),
                    PhysicalQuantity::Vector,
                    FLOORS[0],
                )
                .unwrap();
            assert!(later.global().rms_error > 1000.0 * value.time[0].rms_error);
        }
    }
    assert_eq!(consumer.charged_work(), admission.bounds().work);
    assert_eq!(consumer.remaining(), 0);
    assert_eq!(consumer.next_time(), None);
    assert!(!consumer.is_terminated());
    let spent = consumer.charged_work();
    assert!(consumer.measure(&family).is_err());
    assert_eq!(consumer.charged_work(), spent);
}

#[test]
fn admission_requires_full_joint_storage_floors_and_finite_attempts() {
    let accepted = smooth_family_support::clocks();
    let times = clocks();
    let plan = plan(&accepted, &times, 1e-2);
    let samples = Layout::new([24; 3]).unwrap();
    let admitted = diagnostic(plan, 3);
    let cap = admitted.bounds().joint_storage_bytes;
    assert!(cap > plan.bounds().joint_storage_bytes);
    assert!(ProbeDiagnosticsPlan::new(plan, samples, FLOORS, 3, cap - 1).is_err());
    assert!(
        ProbeDiagnosticsPlan::new(plan, samples, FLOORS, 2, smooth_family_support::CAP).is_err()
    );
    assert!(ProbeDiagnosticsPlan::new(plan, samples, FLOORS, usize::MAX, usize::MAX).is_err());
    assert!(
        ProbeDiagnosticsPlan::new(plan, Layout::new([12; 3]).unwrap(), FLOORS, 3, usize::MAX)
            .is_err()
    );
    for index in 0..6 {
        let mut floors = FLOORS;
        floors[index] = f64::NAN;
        assert!(ProbeDiagnosticsPlan::new(plan, samples, floors, 3, usize::MAX).is_err());
    }
}

#[test]
fn missing_changed_and_skipped_probe_requests_spend_attempts_without_child_work() {
    let accepted = smooth_family_support::clocks();
    let times = clocks();
    let original = plan(&accepted, &times, 1e-2);
    let mut family = ProbeFamily::new(original).unwrap();
    let mut consumer = ProbeDiagnostics::new(diagnostic(original, 5)).unwrap();
    assert!(consumer.measure(&family).is_err());
    let changed = plan(&accepted, &times, 2e-2);
    let mut other = ProbeFamily::new(changed).unwrap();
    other.advance().unwrap();
    assert!(consumer.measure(&other).is_err());
    let altered_times = [
        times[0],
        TickClock::restore(-16, 512, 9, 503).unwrap(),
        times[2],
    ];
    let mut altered = ProbeFamily::new(plan(&accepted, &altered_times, 1e-2)).unwrap();
    altered.advance().unwrap();
    assert!(consumer.measure(&altered).is_err());
    family.advance().unwrap();
    family.advance().unwrap();
    assert!(consumer.measure(&family).is_err());
    assert_eq!(consumer.next_time(), Some(times[0]));
    assert_eq!(consumer.charged_work().attempts, 4);
    assert_eq!(consumer.charged_work().physical.attempts, 0);
    assert_eq!(consumer.charged_work().pressure.attempts, 0);
    assert!(!consumer.is_terminated());
    let mut rejected = ProbeFamily::new(plan(&accepted, &times, 1e-30)).unwrap();
    assert!(rejected.advance().is_err());
    assert!(matches!(
        consumer.measure(&rejected),
        Err(FamilyError::Terminated)
    ));
    assert_eq!(consumer.remaining(), 0);
}
