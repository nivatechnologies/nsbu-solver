//! Fixed bounded startup manifest for the unqualified exact-v2 coordinator.
use nsbu_benchmarks::{
    runtime_force::ForceSettings,
    v2_experiment::{
        diagnostic::{
            AcceptedSchedule, DiagnosticDriver, DiagnosticPlan, DiagnosticSettings,
            ResidualSchedule,
        },
        probes::ProbePlan,
        FamilyPlan, FamilySettings,
    },
    CASE_SHA256,
};
use nsbu_solver::{
    domain::{Layout, TickClock},
    integrators::indicator::Tolerances,
    verification::times::TestedTimes,
};

fn main() {
    let dry_run = arguments();
    let accepted = clocks(&[0, 64, 128]);
    let manifest = clocks(&[0, 7, 63, 64, 95, 127, 128]);
    let residual = [manifest[1], manifest[2], manifest[4], manifest[5]];
    let cap = 256 * 1024 * 1024;
    let family = FamilyPlan::new(
        settings(),
        TestedTimes::new(&accepted, accepted.len()).unwrap(),
        cap,
    )
    .unwrap();
    let probes = ProbePlan::new(
        family,
        TestedTimes::new(&manifest, manifest.len()).unwrap(),
        manifest.len(),
        cap,
    )
    .unwrap();
    let plan = DiagnosticPlan::new(family, probes, &residual, policy(), cap).unwrap();
    println!("case=similarity-mms-v2 sha256={CASE_SHA256}");
    println!(
        "status=UnqualifiedDiagnostic joint_bytes={} coordinator_bytes={} cap={} coordinator_work={:?}",
        plan.bounds().joint_storage_bytes,
        plan.bounds().storage_bytes,
        cap,
        plan.bounds().work,
    );
    println!("component_work={:?}", plan.bounds());
    println!(
        "missing_channels={:?}",
        nsbu_benchmarks::v2_experiment::diagnostic::MISSING_CHANNELS
    );
    if dry_run {
        println!("dry_run=true driver_allocated=false");
        return;
    }
    let mut driver = DiagnosticDriver::new(plan).unwrap();
    while let Some(event) = driver.advance().unwrap() {
        let probe_l2 = event.probe().values().map(|finding| finding.full.l2);
        let residual_l2 = event
            .residual()
            .sample()
            .map(|sample| sample.branches().map(|branch| branch.norms().l2));
        println!(
            "elapsed={} status={:?} accepted={:?} residual={:?} probe_value_L2={probe_l2:?} residual_L2={residual_l2:?}",
            event.clock().elapsed(),
            event.status(),
            event.accepted().schedule(),
            event.residual().schedule(),
        );
        assert_eq!(
            event.accepted().schedule() == AcceptedSchedule::Measured,
            event.residual().schedule() == ResidualSchedule::NotScheduledAtAcceptedClock,
        );
    }
    println!("consumer_charges={:?}", driver.consumer_work());
    println!("accepted_pde_windows=0");
}

fn arguments() -> bool {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [] => false,
        [flag] if flag == "--dry-run" => true,
        _ => panic!("usage: v2_diagnostic_coordinator [--dry-run]"),
    }
}

fn clocks(values: &[u128]) -> Vec<TickClock> {
    values
        .iter()
        .map(|&elapsed| TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap())
        .collect()
}

fn settings() -> FamilySettings {
    FamilySettings {
        grids: [4, 8, 12],
        steps: [64, 32, 16],
        force: ForceSettings {
            samples: Layout::new([12; 3]).unwrap(),
            workers: 0,
        },
        endpoint: 128,
        tolerances: Tolerances {
            absolute: [1e-5, 1e-4],
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
