//! Opt-in cached family/coordinator integration without changing direct profile semantics.
use nsbu_benchmarks::{
    runtime_force::{ForceSettings, IntegrationMode},
    v2_experiment::{
        diagnostic::{DiagnosticDriver, DiagnosticPlan, DiagnosticSettings},
        probes::ProbePlan,
        FamilyPlan, FamilySettings,
    },
    v2_run::{Plan as RunPlan, Run, Settings as RunSettings},
};
use nsbu_solver::{
    domain::{Layout, SpectralState, TickClock},
    experiment::control::Configuration,
    integrators::indicator::Tolerances,
    integrators::{method::Method, trajectory::RunLimits},
    verification::times::TestedTimes,
    SolverError,
};

const CAP: usize = 256 * 1024 * 1024;

fn clocks<const N: usize>(values: [u128; N]) -> [TickClock; N] {
    values.map(|elapsed| TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap())
}

fn run_settings(tolerance: f64) -> RunSettings {
    RunSettings {
        domain: nsbu_solver::domain::Domain::new([4; 3], [1.0; 3], 1.0).unwrap(),
        force: ForceSettings {
            samples: Layout::new([4; 3]).unwrap(),
            workers: 0,
        },
        initial_clock: TickClock::from_rest(-20, 8192).unwrap(),
        configuration: Configuration {
            method: Method::CoxMatthews,
            limits: RunLimits {
                endpoint: 128,
                step_ticks: 128,
                maximum_attempts: 1,
            },
            tolerances: Tolerances {
                absolute: [tolerance; 2],
                relative: [0.0; 2],
            },
        },
        advective_limit: 0.3,
    }
}

fn settings(tolerance: f64) -> FamilySettings {
    FamilySettings {
        grids: [4, 8, 12],
        steps: [32, 16, 8],
        force: ForceSettings {
            samples: Layout::new([12; 3]).unwrap(),
            workers: 0,
        },
        endpoint: 64,
        tolerances: Tolerances {
            absolute: [tolerance, 10.0 * tolerance],
            relative: [0.0; 2],
        },
        advective_limit: 0.3,
    }
}

fn policy() -> DiagnosticSettings {
    DiagnosticSettings {
        physical_samples: Layout::new([12; 3]).unwrap(),
        pressure_samples: Layout::new([24; 3]).unwrap(),
        reference_samples: Layout::new([12; 3]).unwrap(),
        physical_floors: [1e-8, 1e-7, 1e-6, 1e-7],
        pressure_floors: [1e-8, 1e-7],
        reference_floors: [1e-8, 1e-7, 1e-6, 1e-7],
        regional_root_budget: 128,
    }
}

fn plans<'a>(
    accepted: &'a [TickClock],
    manifest: &'a [TickClock],
    residual: &'a [TickClock],
    cached: bool,
) -> DiagnosticPlan<'a> {
    let times = TestedTimes::new(accepted, accepted.len()).unwrap();
    let family = if cached {
        FamilyPlan::new_cached(settings(1e-5), times, CAP).unwrap()
    } else {
        FamilyPlan::new(settings(1e-5), times, CAP).unwrap()
    };
    let probes = ProbePlan::new(
        family,
        TestedTimes::new(manifest, manifest.len()).unwrap(),
        manifest.len(),
        CAP,
    )
    .unwrap();
    DiagnosticPlan::new(family, probes, residual, policy(), CAP).unwrap()
}

fn assert_state_bits(left: &SpectralState, right: &SpectralState) {
    assert_eq!(left.clock(), right.clock());
    assert_eq!(left.epoch(), right.epoch());
    assert_eq!(left.accepted_steps(), right.accepted_steps());
    for axis in 0..3 {
        for (a, b) in left
            .component(axis)
            .unwrap()
            .iter()
            .zip(right.component(axis).unwrap())
        {
            assert_eq!(
                (a.re.to_bits(), a.im.to_bits()),
                (b.re.to_bits(), b.im.to_bits())
            );
        }
    }
}

fn normalized_event(event: nsbu_benchmarks::v2_experiment::diagnostic::DiagnosticEvent) -> String {
    format!("{event:?}")
        .replace(&format!("{:?}", event.family_identity()), "<family>")
        .replace(&format!("{:?}", event.probe_identity()), "<probes>")
}

#[test]
fn cached_and_direct_drivers_publish_bitwise_equal_states_and_events() {
    let accepted = clocks([0, 32, 64]);
    let manifest = clocks([0, 7, 31, 32, 47, 63, 64]);
    let residual = [manifest[1], manifest[2], manifest[4], manifest[5]];
    let direct_plan = plans(&accepted, &manifest, &residual, false);
    let cached_plan = plans(&accepted, &manifest, &residual, true);
    assert_eq!(
        direct_plan.family_plan().integration_mode(),
        IntegrationMode::Direct
    );
    assert_eq!(
        cached_plan.family_plan().integration_mode(),
        IntegrationMode::AttemptCached
    );
    assert_ne!(
        direct_plan.family_plan().identity(),
        cached_plan.family_plan().identity()
    );
    assert_ne!(
        direct_plan.probe_plan().identity(),
        cached_plan.probe_plan().identity()
    );
    assert!(cached_plan.bounds().joint_storage_bytes > direct_plan.bounds().joint_storage_bytes);
    let mut direct = DiagnosticDriver::new(direct_plan).unwrap();
    let mut cached = DiagnosticDriver::new(cached_plan).unwrap();
    for clock in manifest {
        let left = direct.advance().unwrap().unwrap();
        let right = cached.advance().unwrap().unwrap();
        assert_eq!((left.clock(), right.clock()), (clock, clock));
        assert_eq!(normalized_event(left), normalized_event(right));
        for branch in 0..6 {
            assert_state_bits(
                direct.ordinary().branch(branch).unwrap().state(),
                cached.ordinary().branch(branch).unwrap().state(),
            );
            assert_state_bits(
                direct.probes().branch(branch).unwrap().state(),
                cached.probes().branch(branch).unwrap().state(),
            );
        }
    }
    assert!(direct.advance().unwrap().is_none());
    assert!(cached.advance().unwrap().is_none());
    for (branch, calls) in [(0, 12), (5, 15)] {
        assert!(direct
            .ordinary()
            .branch(branch)
            .unwrap()
            .cache_work()
            .is_none());
        let ordinary = cached
            .ordinary()
            .branch(branch)
            .unwrap()
            .cache_work()
            .unwrap();
        let probes = cached
            .probes()
            .branch(branch)
            .unwrap()
            .cache_work()
            .unwrap();
        assert_eq!((ordinary.calls, ordinary.provider_evaluations), (calls, 5));
        assert_eq!((probes.calls, probes.provider_evaluations), (calls, 5));
    }
}

#[test]
fn cached_family_cap_shortfall_and_direct_identity_are_stable() {
    let accepted = clocks([0, 32, 64]);
    let times = TestedTimes::new(&accepted, accepted.len()).unwrap();
    let direct = FamilyPlan::new(settings(1e-5), times, CAP).unwrap();
    let cached = FamilyPlan::new_cached(settings(1e-5), times, CAP).unwrap();
    assert_eq!(direct.integration_mode(), IntegrationMode::Direct);
    assert_eq!(cached.integration_mode(), IntegrationMode::AttemptCached);
    assert!(
        FamilyPlan::new_cached(settings(1e-5), times, cached.bounds().storage_bytes - 1).is_err()
    );
    let standard_clocks = clocks([0, 64, 128]);
    let standard = FamilyPlan::new(
        FamilySettings {
            steps: [64, 32, 16],
            endpoint: 128,
            ..settings(1e-5)
        },
        TestedTimes::new(&standard_clocks, standard_clocks.len()).unwrap(),
        CAP,
    )
    .unwrap();
    let direct_hex = standard
        .identity()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    assert_eq!(
        direct_hex,
        include_str!("fixtures/v2-family-identity-v1.sha256").trim()
    );
}

#[test]
fn pre_rhs_terminal_retry_does_not_reopen_or_reuse_cache_state() {
    let plan = RunPlan::from_rest_cached(run_settings(1e-40), CAP).unwrap();
    let mut run = Run::from_rest(plan).unwrap();
    assert!(matches!(
        run.step().unwrap(),
        nsbu_solver::experiment::control::Outcome::Rejected(_)
    ));
    let work = run.cache_work().unwrap();
    assert_eq!((work.calls, work.provider_evaluations), (12, 5));
    assert_eq!(run.step(), Err(SolverError::RetryLimit));
    assert_eq!(run.cache_work(), Some(work));
}
