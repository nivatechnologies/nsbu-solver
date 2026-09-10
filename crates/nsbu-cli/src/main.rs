//! NSBU Solver command-line entry point.
mod arguments;

use arguments::{parse, Args, Command, MethodName};
use nsbu_benchmarks::smooth_observer::{
    BalanceObserver, BalanceObserverLimits, BalanceObserverWork,
};
use nsbu_benchmarks::smooth_run::{IntegrationWork, SmoothPlan, SmoothRun};
use nsbu_solver::{
    domain::{Domain, TickClock},
    experiment::control::Configuration,
    integrators::{
        indicator::Tolerances,
        method::Method,
        trajectory::{RunLimits, StopReason},
    },
    SolverError,
};
use std::process::ExitCode;

const DOMAIN_LENGTH: f64 = 1.0;
const VISCOSITY: f64 = 0.3;
const ADVECTIVE_LIMIT: f64 = 1.0;

fn main() -> ExitCode {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    match parse(&arguments) {
        Command::Help => print_help(),
        Command::Version => println!("{} {}", nsbu_solver::NAME, nsbu_solver::VERSION),
        Command::Smooth(args) => return smooth(args),
        Command::Invalid => {
            eprintln!("Unsupported arguments. Use nsbu --help for available commands.");
            return ExitCode::from(2);
        }
    }
    ExitCode::SUCCESS
}

fn print_help() {
    println!(
        "{}\nUsage: nsbu [--help | --version]\n       nsbu smooth [OPTIONS]\n\nBuilt-in profile: CyclicSine on [1.0, 1.0, 1.0], viscosity 0.3, advective guard 1.0.\n\nOptions:\n  --grid, --domain N          Four-multiple cubic grid (default: 8)\n  --method cm|ho              Cox-Matthews or Hochbruck-Ostermann (default: cm)\n  --step-ticks, --step N      Exact macro-step ticks (default: 64)\n  --target, --endpoint-ticks N\n                              Requested exact endpoint ticks (default: 256)\n  --tick-exponent, --tick N   Tick quantum exponent for 2^N (default: -16)\n  --attempts, --attempt-cap N Maximum recorded attempts (default: 4)\n  --memory-cap N              Declared byte cap (default: 67108864)\n  --dry-run                   Validate and report the allocation-free admission plan\n\nThe built-in CyclicSine profile is a numerical diagnostic only. It is not PDE-qualified.",
        nsbu_solver::NAME
    );
}

fn smooth(args: Args) -> ExitCode {
    let configuration = configuration(&args);
    let domain = match Domain::new([args.grid; 3], [DOMAIN_LENGTH; 3], VISCOSITY) {
        Ok(value) => value,
        Err(error) => return refused(error),
    };
    let clock = match clock(&args) {
        Ok(value) => value,
        Err(error) => return refused(error),
    };
    let samples = match args.maximum_attempts.checked_add(1) {
        Some(value) => value,
        None => return refused(SolverError::SizeOverflow),
    };
    let plan = match SmoothPlan::from_rest(
        domain,
        clock,
        configuration,
        samples,
        ADVECTIVE_LIMIT,
        args.memory_cap,
    ) {
        Ok(value) => value,
        Err(error) => return refused(error),
    };
    if args.dry_run {
        print_plan(args, clock, plan);
        return ExitCode::SUCCESS;
    }
    execute(args, clock, configuration, samples)
}

fn configuration(args: &Args) -> Configuration {
    Configuration {
        limits: RunLimits {
            endpoint: args.endpoint_ticks,
            step_ticks: args.step_ticks,
            maximum_attempts: args.maximum_attempts,
        },
        method: method(args.method),
        tolerances: Tolerances {
            absolute: [1.0e-2; 2],
            relative: [0.0; 2],
        },
    }
}

fn method(name: MethodName) -> Method {
    match name {
        MethodName::CoxMatthews => Method::CoxMatthews,
        MethodName::HochbruckOstermann => Method::HochbruckOstermann,
    }
}

fn clock(args: &Args) -> Result<TickClock, SolverError> {
    let margin = args
        .step_ticks
        .checked_mul(2)
        .ok_or(SolverError::ClockCapacityExceeded)?;
    let target = args
        .endpoint_ticks
        .checked_add(margin)
        .ok_or(SolverError::ClockCapacityExceeded)?;
    TickClock::from_rest(args.tick_exponent, target)
}

fn execute(args: Args, clock: TickClock, configuration: Configuration, samples: usize) -> ExitCode {
    let domain = match Domain::new([args.grid; 3], [DOMAIN_LENGTH; 3], VISCOSITY) {
        Ok(value) => value,
        Err(error) => return refused(error),
    };
    let mut run = match SmoothRun::from_rest(
        domain,
        clock,
        configuration,
        samples,
        ADVECTIVE_LIMIT,
        args.memory_cap,
    ) {
        Ok(value) => value,
        Err(error) => return refused(error),
    };
    while run.history().controller().stopped().is_none() {
        if let Err(error) = run.step() {
            return refused(error);
        }
    }
    print_run(args, clock, &run);
    match run.history().controller().stopped() {
        Some(StopReason::EndpointReached) => ExitCode::SUCCESS,
        Some(_) | None => ExitCode::from(1),
    }
}

fn print_plan(args: Args, clock: TickClock, plan: SmoothPlan) {
    let resources = plan.resources();
    let observer = match BalanceObserver::limits(resources.domain(), plan.observer_samples()) {
        Ok(value) => value,
        Err(error) => {
            let _ = refused(error);
            return;
        }
    };
    println!(
        concat!(
            "{{\"status\":\"dry_run\",\"profile\":{{\"name\":\"CyclicSine\",",
            "\"lengths\":[1.0,1.0,1.0],\"viscosity\":0.3,\"pde_qualified\":false}},",
            "\"qualification_status\":\"unqualified\",\"origin_status\":\"internal_from_rest\",",
            "\"method\":\"{}\",\"clock\":{},\"tolerances\":{}",
            ",\"memory\":{{\"classes\":{},\"total_bytes\":\"{}\"}}",
            ",\"bounded_integration\":{{\"rhs_calls\":\"{}\",",
            "\"work_units\":\"{}\",\"scalar_transforms\":\"{}\"}},",
            "\"bounded_observer\":{}}}"
        ),
        method_name(args.method),
        clock_json(args, clock, "0"),
        tolerances_json(),
        classes_json(resources.classes()),
        resources.total(),
        plan.integration_calls(),
        plan.integration_work_units(),
        plan.integration_scalar_transforms(),
        observer_limits_json(observer),
    );
}

fn print_run(args: Args, clock: TickClock, run: &SmoothRun) {
    let controller = run.history().controller();
    let work = total_work(run.work());
    let observer = run.observer_work();
    let (status, stop_reason) = match controller.stopped() {
        Some(StopReason::EndpointReached) => ("completed", "endpoint_reached"),
        Some(StopReason::LocalErrorRejected) => ("failed", "local_error_rejected"),
        Some(StopReason::Refused(error)) => ("failed", error_name(error)),
        None => ("failed", "not_stopped"),
    };
    println!(
        concat!(
            "{{\"status\":\"{}\",\"profile\":{{\"name\":\"CyclicSine\",",
            "\"lengths\":[1.0,1.0,1.0],\"viscosity\":0.3,\"pde_qualified\":false}},",
            "\"qualification_status\":\"unqualified\",\"origin_status\":\"internal_from_rest\",",
            "\"method\":\"{}\",\"stop_reason\":\"{}\",\"clock\":{},",
            "\"tolerances\":{},\"attempts\":{{\"started\":\"{}\",\"committed\":\"{}\"}},",
            "\"integration\":{{\"rhs_calls\":\"{}\",\"work_units\":\"{}\",",
            "\"scalar_transforms\":\"{}\"}},\"diagnostics\":{} }}"
        ),
        status,
        method_name(args.method),
        stop_reason,
        clock_json(args, clock, &controller.clock().elapsed().to_string()),
        tolerances_json(),
        controller.attempted(),
        controller.committed(),
        work.0,
        work.1,
        work.2,
        diagnostics_json(run, observer),
    );
}

fn clock_json(args: Args, clock: TickClock, elapsed: &str) -> String {
    format!(
        "{{\"tick_exponent\":{},\"requested_endpoint_ticks\":\"{}\",\"actual_elapsed_ticks\":\"{}\",\"step_ticks\":\"{}\",\"clock_target_ticks\":\"{}\"}}",
        args.tick_exponent, args.endpoint_ticks, elapsed, args.step_ticks, clock.target()
    )
}

fn classes_json(classes: [usize; 8]) -> String {
    let values = classes.map(|value| format!("\"{value}\""));
    format!("[{}]", values.join(","))
}

fn total_work(work: &[IntegrationWork]) -> (u128, u128, u128) {
    // A run admits at most `usize::MAX` entries, each holding `usize` charges. Their
    // independent products fit in u128, which preserves the checked source accounting.
    work.iter().fold((0, 0, 0), |totals, entry| {
        (
            totals.0 + entry.calls() as u128,
            totals.1 + entry.work_units() as u128,
            totals.2 + entry.scalar_transforms() as u128,
        )
    })
}

fn tolerances_json() -> &'static str {
    "{\"absolute\":[0.01,0.01],\"relative\":[0.0,0.0],\"advective_guard\":1.0}"
}

fn observer_limits_json(limits: BalanceObserverLimits) -> String {
    format!(
        "{{\"sample_cap\":\"{}\",\"work_units\":\"{}\",\"scalar_transforms\":\"{}\"}}",
        limits.samples, limits.work_units, limits.scalar_transforms
    )
}

fn diagnostics_json(run: &SmoothRun, work: BalanceObserverWork) -> String {
    let sample = run
        .history()
        .records()
        .iter()
        .rev()
        .find_map(|record| record.sample);
    let values = match sample {
        Some(value) => format!(
            "{{\"sampled\":true,\"energy\":{},\"enstrophy\":{}}}",
            value.energy, value.enstrophy
        ),
        None => "{\"sampled\":false,\"energy\":null,\"enstrophy\":null}".to_owned(),
    };
    format!(
        "{{\"pending_midpoint\":{},\"last_accepted_sample\":{},\"observer_charges\":{{\"samples\":\"{}\",\"work_units\":\"{}\",\"scalar_transforms\":\"{}\"}}}}",
        run.history().balance().has_pending_midpoint(),
        values,
        work.samples,
        work.work_units,
        work.scalar_transforms,
    )
}

fn method_name(method: MethodName) -> &'static str {
    match method {
        MethodName::CoxMatthews => "cm",
        MethodName::HochbruckOstermann => "ho",
    }
}

fn refused(error: SolverError) -> ExitCode {
    println!(
        "{{\"status\":\"refused\",\"qualification_status\":\"unqualified\",\"origin_status\":\"internal_from_rest\",\"profile\":{{\"name\":\"CyclicSine\",\"pde_qualified\":false}},\"error\":\"{}\"}}",
        error_name(error)
    );
    ExitCode::from(1)
}

fn error_name(error: SolverError) -> &'static str {
    match error {
        SolverError::InvalidDomain => "invalid_domain",
        SolverError::InvalidIndex => "invalid_index",
        SolverError::SizeOverflow => "size_overflow",
        SolverError::InvalidClock => "invalid_clock",
        SolverError::InvalidStep => "invalid_step",
        SolverError::ClockCapacityExceeded => "clock_capacity_exceeded",
        SolverError::EpochExhausted => "epoch_exhausted",
        SolverError::ResourceLimit => "resource_limit",
        SolverError::AllocationFailed => "allocation_failed",
        SolverError::InvalidPayload => "invalid_payload",
        SolverError::InvalidSpectrum => "invalid_spectrum",
        SolverError::ArithmeticResolutionLimited => "arithmetic_resolution_limited",
        SolverError::StaleAttempt => "stale_attempt",
        SolverError::RetryLimit => "retry_limit",
        SolverError::UnknownProviderCost => "unknown_provider_cost",
        SolverError::ProviderBudgetExceeded => "provider_budget_exceeded",
        SolverError::AdvectiveLimit => "advective_limit",
    }
}
