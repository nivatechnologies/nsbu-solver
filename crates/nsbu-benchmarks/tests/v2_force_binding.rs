//! Raw spectral force-resolution evidence bound to an independent ordinary baseline.
mod v2_family_oracle;
use nsbu_benchmarks::{
    runtime_force::ForceSettings,
    v2_experiment::{FamilyPlan, FamilySettings, V2Family},
    v2_force_experiment::{
        binding::{
            ForceBindingError, ForceBindingPlan, ForceBindingWorkspace, ForceResolutionStatus,
        },
        ForceFamily, ForceFamilyPlan, ForceFamilySettings,
    },
    CASE_SHA256,
};
use nsbu_solver::{
    domain::{Layout, TickClock},
    integrators::{indicator::Tolerances, method::Method},
    verification::times::TestedTimes,
};
use v2_family_oracle::{assert_oracle, modal_oracle};

const CAP: usize = 256 * 1024 * 1024;

fn clocks() -> [TickClock; 3] {
    [0, 64, 128].map(|t| TickClock::restore(-20, 8192, t, 8192 - t).unwrap())
}
fn tolerances(scale: f64) -> Tolerances {
    Tolerances {
        absolute: [scale, 10.0 * scale],
        relative: [0.0; 2],
    }
}
fn force_settings(scale: f64) -> ForceFamilySettings {
    ForceFamilySettings {
        grid: 4,
        force_grids: [6, 12, 24],
        workers: 0,
        step_ticks: 16,
        method: Method::CoxMatthews,
        endpoint: 128,
        tolerances: tolerances(scale),
        advective_limit: 0.3,
    }
}
fn ordinary_settings(scale: f64) -> FamilySettings {
    FamilySettings {
        grids: [4, 8, 12],
        steps: [64, 32, 16],
        force: ForceSettings {
            samples: Layout::new([12; 3]).unwrap(),
            workers: 0,
        },
        endpoint: 128,
        tolerances: tolerances(scale),
        advective_limit: 0.3,
    }
}

#[test]
fn independent_baseline_words_bind_raw_spectral_force_differences() {
    let times = clocks();
    let tested = TestedTimes::new(&times, times.len()).unwrap();
    let force_plan = ForceFamilyPlan::new(force_settings(1e-5), tested, CAP).unwrap();
    let ordinary_plan = FamilyPlan::new(ordinary_settings(1e-5), tested, CAP).unwrap();
    let plan = ForceBindingPlan::new(force_plan, ordinary_plan, 1, 0, CAP).unwrap();
    assert_eq!(plan.baseline_slots(), (1, 0));
    assert_eq!(plan.bounds().attempts, times.len());
    assert_eq!(plan.bounds().coefficient_words, times.len() * 6 * 4 * 4 * 3);
    let mut force = ForceFamily::from_rest(force_plan).unwrap();
    let mut ordinary = V2Family::new(ordinary_plan).unwrap();
    let mut binding = ForceBindingWorkspace::new(plan);
    let mut last = None;
    for time in times {
        ordinary.advance().unwrap().unwrap();
        let raw = force.advance().unwrap().unwrap();
        last = Some(raw);
        let report = binding.measure(&force, &ordinary, raw).unwrap();
        assert_eq!(report.clock(), time);
        assert_eq!(report.force_identity(), force_plan.identity());
        assert_eq!(report.ordinary_identity(), ordinary_plan.identity());
        assert_eq!(report.baseline_slots(), (1, 0));
        assert_eq!(report.case_sha256(), CASE_SHA256);
        assert_eq!(report.status(), ForceResolutionStatus::DiagnosticOnly);
        assert_eq!(report.force_settings().force_grids, [6, 12, 24]);
        check_oracle(&force, report.comparisons());
    }
    assert_eq!(binding.current().unwrap().clock().elapsed(), 128);
    assert_eq!(binding.charged_work(), (3, plan.bounds().coefficient_words));
    assert!(matches!(
        binding.measure(&force, &ordinary, last.unwrap()),
        Err(ForceBindingError::Numerical(_))
    ));
}

fn check_oracle(
    family: &ForceFamily<'_>,
    actual: [nsbu_solver::diagnostics::comparison::BandComparison; 2],
) {
    for (slot, (a, b)) in [(0, 1), (1, 2)].into_iter().enumerate() {
        let left = family.branch(a).unwrap();
        let right = family.branch(b).unwrap();
        let expected = modal_oracle(
            left.plan().settings().domain,
            right.plan().settings().domain,
            std::array::from_fn(|axis| left.state().component(axis).unwrap()),
            std::array::from_fn(|axis| right.state().component(axis).unwrap()),
        );
        assert_oracle(actual[slot], expected);
    }
}

#[test]
fn foreign_stale_and_insufficient_bindings_publish_nothing_and_terminate() {
    let times = clocks();
    let tested = TestedTimes::new(&times, times.len()).unwrap();
    let force_plan = ForceFamilyPlan::new(force_settings(1e-5), tested, CAP).unwrap();
    let ordinary_plan = FamilyPlan::new(ordinary_settings(1e-5), tested, CAP).unwrap();
    let admitted = ForceBindingPlan::new(force_plan, ordinary_plan, 1, 0, CAP).unwrap();
    assert!(ForceBindingPlan::new(
        force_plan,
        ordinary_plan,
        1,
        0,
        admitted.bounds().joint_storage_bytes - 1
    )
    .is_err());
    assert!(ForceBindingPlan::new(force_plan, ordinary_plan, 0, 0, CAP).is_err());
    assert!(ForceBindingPlan::new(force_plan, ordinary_plan, 1, 9, CAP).is_err());

    let foreign_plan = FamilyPlan::new(ordinary_settings(2e-5), tested, CAP).unwrap();
    let mut force = ForceFamily::from_rest(force_plan).unwrap();
    let foreign = V2Family::new(foreign_plan).unwrap();
    let raw = force.advance().unwrap().unwrap();
    let mut binding = ForceBindingWorkspace::new(admitted);
    assert!(matches!(
        binding.measure(&force, &foreign, raw),
        Err(ForceBindingError::InvalidBinding)
    ));
    assert!(binding.current().is_none());
    assert_eq!(
        binding.charged_work(),
        (1, admitted.bounds().coefficient_words / 3)
    );
    assert!(matches!(
        binding.measure(&force, &foreign, raw),
        Err(ForceBindingError::Terminated)
    ));

    let mut force = ForceFamily::from_rest(force_plan).unwrap();
    let ordinary = V2Family::new(ordinary_plan).unwrap();
    let first = force.advance().unwrap().unwrap();
    let mut stale = ForceBindingWorkspace::new(admitted);
    assert!(matches!(
        stale.measure(&force, &ordinary, first),
        Err(ForceBindingError::InvalidBinding)
    ));
    assert!(stale.current().is_none());

    let mut force = ForceFamily::from_rest(force_plan).unwrap();
    let mut ordinary = V2Family::new(ordinary_plan).unwrap();
    ordinary.advance().unwrap().unwrap();
    let first = force.advance().unwrap().unwrap();
    let mut stale = ForceBindingWorkspace::new(admitted);
    stale.measure(&force, &ordinary, first).unwrap();
    let second = force.advance().unwrap().unwrap();
    assert!(matches!(
        stale.measure(&force, &ordinary, second),
        Err(ForceBindingError::InvalidBinding)
    ));
    assert_eq!(stale.current().unwrap().clock().elapsed(), 0);
}

#[test]
fn terminal_force_owner_cannot_rebind_and_prior_report_survives() {
    let times = clocks();
    let tested = TestedTimes::new(&times, times.len()).unwrap();
    let force_plan = ForceFamilyPlan::new(force_settings(1e-40), tested, CAP).unwrap();
    let ordinary_plan = FamilyPlan::new(ordinary_settings(1e-40), tested, CAP).unwrap();
    let plan = ForceBindingPlan::new(force_plan, ordinary_plan, 1, 0, CAP).unwrap();
    let mut force = ForceFamily::from_rest(force_plan).unwrap();
    let mut ordinary = V2Family::new(ordinary_plan).unwrap();
    ordinary.advance().unwrap().unwrap();
    let rest = force.advance().unwrap().unwrap();
    let mut binding = ForceBindingWorkspace::new(plan);
    binding.measure(&force, &ordinary, rest).unwrap();
    assert!(force.advance().is_err());
    assert!(matches!(
        binding.measure(&force, &ordinary, rest),
        Err(ForceBindingError::InvalidBinding)
    ));
    assert_eq!(binding.current().unwrap().clock().elapsed(), 0);
    assert!(matches!(
        binding.measure(&force, &ordinary, rest),
        Err(ForceBindingError::Terminated)
    ));
}
