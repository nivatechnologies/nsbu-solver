//! Fixed-profile parsing and concise unqualified diagnostic reporting.
use nsbu_benchmarks::v2_experiment::{
    binding::NodeBindingStatus,
    diagnostic::{DiagnosticDriver, DiagnosticEvent, StartupProfile},
    physical, pressure,
};
use std::{ffi::OsString, process::ExitCode};

const CAP: usize = 256 * 1024 * 1024;

enum Invocation {
    Help,
    Run { dry_run: bool },
    Invalid,
}

pub(crate) fn dispatch(arguments: &[OsString]) -> Option<ExitCode> {
    if arguments.first()? != "diagnose-v2" {
        return None;
    }
    Some(match parse(&arguments[1..]) {
        Invocation::Help => {
            help();
            ExitCode::SUCCESS
        }
        Invocation::Run { dry_run } => execute(dry_run),
        Invocation::Invalid => {
            eprintln!("Unsupported diagnose-v2 arguments. Use nsbu diagnose-v2 --help.");
            ExitCode::from(2)
        }
    })
}

fn parse(arguments: &[OsString]) -> Invocation {
    match arguments {
        [] => Invocation::Run { dry_run: false },
        [flag] if flag == "--dry-run" => Invocation::Run { dry_run: true },
        [flag] if flag == "--help" || flag == "-h" => Invocation::Help,
        _ => Invocation::Invalid,
    }
}

fn help() {
    println!(
        "NSBU Solver fixed exact-v2 startup diagnostic\n\
Usage: nsbu diagnose-v2 [--dry-run]\n\n\
The fixed profile uses N=4/8/12, steps 64/32/16 and clocks\n\
0,7,63,64,95,127,128. --dry-run admits resources without allocating the driver.\n\
Every result is UnqualifiedDiagnostic; no PDE window is qualified."
    );
}

fn execute(dry_run: bool) -> ExitCode {
    let profile = match StartupProfile::new() {
        Ok(value) => value,
        Err(error) => return refused(&format!("{error:?}")),
    };
    let plan = match profile.plan(CAP) {
        Ok(value) => value,
        Err(error) => return refused(&format!("{error:?}")),
    };
    let family = plan.family_plan().settings();
    let diagnostic = plan.diagnostic_settings();
    println!(
        "{{\"command\":\"diagnose-v2\",\"status\":\"admitted\",\"scientific_status\":\"UnqualifiedDiagnostic\",\"case\":\"similarity-mms-v2\",\"case_sha256\":\"{}\",\"family_identity\":\"{}\",\"probe_identity\":\"{}\",\"grids\":{:?},\"steps\":{:?},\"branch_profiles\":[[4,16,\"cm\"],[8,16,\"cm\"],[12,16,\"cm\"],[12,64,\"cm\"],[12,32,\"cm\"],[12,16,\"ho\"]],\"force_grid\":{},\"workers\":{},\"tick_exponent\":-20,\"clock_target_ticks\":\"8192\",\"accepted_clocks\":[\"0\",\"64\",\"128\"],\"probe_clocks\":[\"0\",\"7\",\"63\",\"64\",\"95\",\"127\",\"128\"],\"physical_samples\":{:?},\"reference_samples\":{:?},\"pressure_samples\":{:?},\"memory_cap\":\"{}\",\"joint_storage_bytes\":\"{}\",\"coordinator_storage_bytes\":\"{}\",\"event_attempts\":\"{}\",\"accepted_events\":\"{}\",\"residual_events\":\"{}\"}}",
        nsbu_benchmarks::CASE_SHA256,
        hex(plan.family_plan().identity()),
        hex(plan.probe_plan().identity()),
        family.grids,
        family.steps,
        family.force.samples.dimensions()[0],
        family.force.workers,
        diagnostic.physical_samples.dimensions(),
        diagnostic.reference_samples.dimensions(),
        diagnostic.pressure_samples.dimensions(),
        CAP,
        plan.bounds().joint_storage_bytes,
        plan.bounds().storage_bytes,
        plan.bounds().work.attempts,
        plan.bounds().work.accepted_events,
        plan.bounds().work.residual_events,
    );
    print_work(plan.bounds());
    println!("{{\"missing_channels\":[\"force_resolution\",\"force_precision\",\"arithmetic\",\"reference_precision\",\"pressure_reference\",\"pressure_gauge\",\"transfer\",\"sampling_resolution\",\"quadrature_resolution\",\"region_volume_coverage\"]}}");
    if dry_run {
        println!("{{\"command\":\"diagnose-v2\",\"status\":\"dry_run\",\"driver_allocated\":false,\"qualified_windows\":0,\"pde_qualified\":false}}");
        return ExitCode::SUCCESS;
    }
    run(plan)
}

fn print_work(bounds: nsbu_benchmarks::v2_experiment::diagnostic::DiagnosticBounds) {
    println!(
        "{{\"type\":\"work_preflight\",\"probe_weighted_visits\":\"{}\",\"physical_scalar_transforms\":\"{}\",\"physical_weighted_visits\":\"{}\",\"pressure_provider_work\":\"{}\",\"pressure_scalar_transforms\":\"{}\",\"pressure_weighted_visits\":\"{}\",\"reference_evaluations\":\"{}\",\"reference_root_iterations\":\"{}\",\"reference_scalar_transforms\":\"{}\",\"reference_weighted_visits\":\"{}\",\"regional_classifications\":\"{}\",\"regional_root_iterations\":\"{}\",\"residual_provider_work\":\"{}\",\"residual_scalar_transforms\":\"{}\",\"residual_coefficient_work\":\"{}\",\"binding_node_lookups\":\"{}\",\"binding_coefficient_visits\":\"{}\"}}",
        bounds.probes.weighted_visits,
        bounds.physical.scalar_transforms,
        bounds.physical.weighted_visits,
        bounds.pressure.provider_work_units,
        bounds.pressure.scalar_transforms,
        bounds.pressure.weighted_visits,
        bounds.reference.reference_evaluations,
        bounds.reference.root_iterations,
        bounds.reference.scalar_transforms,
        bounds.reference.weighted_visits,
        bounds.regional.classifications,
        bounds.regional.root_iterations,
        bounds.residual.residual.provider_work_units,
        bounds.residual.residual.scalar_transforms,
        bounds.residual.residual.coefficient_work_units,
        bounds.binding.node_lookups,
        bounds.binding.coefficient_visits,
    );
}

fn run(plan: nsbu_benchmarks::v2_experiment::diagnostic::DiagnosticPlan<'_>) -> ExitCode {
    let mut driver = match DiagnosticDriver::new(plan) {
        Ok(value) => value,
        Err(error) => return refused(&format!("{error:?}")),
    };
    loop {
        match driver.advance() {
            Ok(Some(event)) => print_event(event),
            Ok(None) => break,
            Err(error) => return refused(&format!("{error:?}")),
        }
    }
    println!(
        "{{\"command\":\"diagnose-v2\",\"status\":\"completed\",\"scientific_status\":\"UnqualifiedDiagnostic\",\"events\":{},\"qualified_windows\":0,\"pde_qualified\":false}}",
        driver.reports().len()
    );
    ExitCode::SUCCESS
}

fn print_event(event: DiagnosticEvent) {
    let probe = event
        .probe()
        .values()
        .iter()
        .map(|finding| finding.full.l2)
        .fold(0.0_f64, f64::max);
    if let Some(sample) = event.accepted().sample() {
        let physical = physical_maxima(sample.physical);
        let pressure = pressure_maxima(sample.pressure);
        let reference = reference_maxima(sample.regional_reference);
        let equal = sample
            .node_binding
            .branches()
            .iter()
            .filter(|finding| matches!(finding, NodeBindingStatus::Compared(node) if node.coefficients_equal))
            .count();
        println!(
            "{{\"type\":\"diagnostic_summary\",\"scientific_status\":\"UnqualifiedDiagnostic\",\"clock_ticks\":\"{}\",\"path\":\"accepted\",\"probe_pair_max_l2\":{probe:.17e},\"physical_pair_max_rms\":{{\"velocity\":{:.17e},\"gradient\":{:.17e},\"hessian\":{:.17e},\"vorticity\":{:.17e}}},\"pressure_pair_max_rms\":{{\"pressure\":{:.17e},\"pressure_gradient\":{:.17e}}},\"reference_branch_max_rms\":{{\"velocity\":{:.17e},\"gradient\":{:.17e},\"hessian\":{:.17e},\"vorticity\":{:.17e}}},\"bitwise_equal_nodes\":{equal}}}",
            event.clock().elapsed(),
            physical[0],
            physical[1],
            physical[2],
            physical[3],
            pressure[0],
            pressure[1],
            reference[0],
            reference[1],
            reference[2],
            reference[3],
        );
    } else if let Some(sample) = event.residual().sample() {
        let residual = sample
            .branches()
            .iter()
            .map(|branch| branch.norms().l2)
            .fold(0.0_f64, f64::max);
        println!(
            "{{\"type\":\"diagnostic_summary\",\"scientific_status\":\"UnqualifiedDiagnostic\",\"clock_ticks\":\"{}\",\"path\":\"residual\",\"probe_max_l2\":{probe:.17e},\"residual_max_l2\":{residual:.17e}}}",
            event.clock().elapsed(),
        );
    }
}

fn physical_maxima(sample: physical::PhysicalRefinementSample) -> [f64; 4] {
    sample
        .quantities()
        .map(|quantity| local_max(quantity.space, quantity.time, quantity.method))
}
fn pressure_maxima(sample: pressure::PressureRefinementSample) -> [f64; 2] {
    sample
        .quantities()
        .map(|quantity| local_max(quantity.space, quantity.time, quantity.method))
}
fn local_max(
    space: [nsbu_solver::diagnostics::local::LocalError; 2],
    time: [nsbu_solver::diagnostics::local::LocalError; 2],
    method: nsbu_solver::diagnostics::local::LocalError,
) -> f64 {
    space
        .into_iter()
        .chain(time)
        .chain([method])
        .map(|error| error.rms_error)
        .fold(0.0, f64::max)
}
fn reference_maxima(
    sample: nsbu_benchmarks::v2_experiment::reference::regional::RegionalTrackingSample,
) -> [f64; 4] {
    std::array::from_fn(|quantity| {
        sample
            .branches()
            .iter()
            .map(|branch| branch.quantities[quantity].global.rms_error)
            .fold(0.0, f64::max)
    })
}

fn hex(bytes: [u8; 32]) -> String {
    use std::fmt::Write;
    let mut result = String::with_capacity(64);
    for byte in bytes {
        write!(&mut result, "{byte:02x}").expect("writing to String cannot fail");
    }
    result
}

fn refused(reason: &str) -> ExitCode {
    println!(
        "{{\"command\":\"diagnose-v2\",\"status\":\"refused\",\"reason\":\"{reason}\",\"qualified_windows\":0,\"pde_qualified\":false}}"
    );
    ExitCode::from(1)
}
