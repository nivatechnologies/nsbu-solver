//! Fixed bounded startup manifest for the unqualified exact-v2 coordinator.
use nsbu_benchmarks::{
    v2_experiment::diagnostic::{
        AcceptedSchedule, DiagnosticDriver, DiagnosticPlan, ResidualSchedule, StartupProfile,
    },
    CASE_SHA256,
};

fn main() {
    execute(arguments());
}

fn execute(dry_run: bool) {
    execute_with(dry_run, run);
}

fn execute_with(dry_run: bool, runner: fn(DiagnosticPlan<'_>)) {
    let cap = 256 * 1024 * 1024;
    let profile = StartupProfile::new().unwrap();
    let plan = profile.plan(cap).unwrap();
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
    runner(plan);
}

fn run(plan: DiagnosticPlan<'_>) {
    let mut driver = DiagnosticDriver::new(plan).unwrap();
    while let Some(event) = driver.advance().unwrap() {
        print_event(event);
    }
    println!("consumer_charges={:?}", driver.consumer_work());
    println!("accepted_pde_windows=0");
}

fn print_event(event: nsbu_benchmarks::v2_experiment::diagnostic::DiagnosticEvent) {
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

fn arguments() -> bool {
    let arguments = std::env::args().skip(1).collect::<Vec<_>>();
    parse_arguments(&arguments)
        .unwrap_or_else(|| panic!("usage: v2_diagnostic_coordinator [--dry-run]"))
}

fn parse_arguments(arguments: &[String]) -> Option<bool> {
    match arguments {
        [] => Some(false),
        [flag] if flag == "--dry-run" => Some(true),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::{execute, execute_with, parse_arguments};

    #[test]
    fn parser_and_dry_run_cover_the_bounded_example_boundary() {
        assert_eq!(parse_arguments(&[]), Some(false));
        assert_eq!(parse_arguments(&["--dry-run".into()]), Some(true));
        assert_eq!(parse_arguments(&["--bad".into()]), None);
        assert_eq!(parse_arguments(&["--dry-run".into(), "extra".into()]), None);
        execute(true);
        execute_with(false, |_| {});
    }
}
