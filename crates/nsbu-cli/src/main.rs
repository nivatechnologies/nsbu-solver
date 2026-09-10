//! NSBU Solver command-line entry point.
mod arguments;
mod checkpoint_io;
mod report;
use report::{
    checkpoint_refused, io_refused, origin_name, print_checkpoint, print_plan, print_run, refused,
    refused_with_origin,
};

use arguments::{parse, Args, Command, MethodName};
use nsbu_benchmarks::smooth_run::{SmoothPlan, SmoothRun};
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
use std::{path::Path, process::ExitCode};

const DOMAIN_LENGTH: f64 = 1.0;
const VISCOSITY: f64 = 0.3;
const ADVECTIVE_LIMIT: f64 = 1.0;

fn main() -> ExitCode {
    let arguments = std::env::args_os().skip(1).collect::<Vec<_>>();
    match parse(&arguments) {
        Command::Help => print_help(),
        Command::Version => println!("{} {}", nsbu_solver::NAME, nsbu_solver::VERSION),
        Command::Smooth(args) => return smooth(args),
        Command::Resume(args) => return resume(args),
        Command::Invalid => {
            eprintln!("Unsupported arguments. Use nsbu --help for available commands.");
            return ExitCode::from(2);
        }
    }
    ExitCode::SUCCESS
}

fn print_help() {
    println!(
        "{}\nUsage: nsbu [--help | --version]\n       nsbu smooth [OPTIONS]\n       nsbu resume --checkpoint PATH [OPTIONS]\n\nBuilt-in profile: CyclicSine on [1.0, 1.0, 1.0], viscosity 0.3, advective guard 1.0.\n\nOptions:\n  --grid, --domain N          Four-multiple cubic grid (default: 8)\n  --method cm|ho              Cox-Matthews or Hochbruck-Ostermann (default: cm)\n  --step-ticks, --step N      Exact macro-step ticks (default: 64)\n  --target, --endpoint-ticks N\n                              Requested exact endpoint ticks (default: 256)\n  --tick-exponent, --tick N   Tick quantum exponent for 2^N (default: -16)\n  --attempts, --attempt-cap N Maximum recorded attempts (default: 4)\n  --memory-cap N              Declared byte cap (default: 67108864)\n  --checkpoint PATH           Save after --checkpoint-after accepted steps, or resume this file\n  --checkpoint-after N        Accepted-step count at which smooth saves and exits\n  --dry-run                   Validate and report the allocation-free admission plan\n\nThe built-in CyclicSine profile is a numerical diagnostic only. It is not PDE-qualified.",
        nsbu_solver::NAME
    );
}

fn smooth(args: Args) -> ExitCode {
    let (clock, configuration, samples, plan) = match admit(&args) {
        Ok(value) => value,
        Err(error) => return refused(error),
    };
    if let Some(target) = args.checkpoint_after {
        if target as u128 > configuration.limits.endpoint / configuration.limits.step_ticks {
            return io_refused("checkpoint_step_out_of_range", "internal_from_rest");
        }
    }
    let checkpoint_bytes = if args.checkpoint.is_some() {
        match checkpoint_preflight(plan, configuration, args.memory_cap) {
            Ok(value) => value,
            Err(error) => return refused(error),
        }
    } else {
        0
    };
    if args.dry_run {
        print_plan(args, clock, plan, checkpoint_bytes);
        return ExitCode::SUCCESS;
    }
    if let Some(path) = args.checkpoint.clone() {
        return execute_checkpoint(args, clock, configuration, samples, &path, checkpoint_bytes);
    }
    execute(args, clock, configuration, samples)
}

fn checkpoint_preflight(
    plan: SmoothPlan,
    configuration: Configuration,
    cap: usize,
) -> Result<usize, SolverError> {
    let bytes =
        nsbu_benchmarks::smooth_run::archive::maximum_encoded_len(plan.resources(), configuration)
            .map_err(|_| SolverError::ResourceLimit)?;
    plan.resources()
        .total()
        .checked_add(bytes)
        .filter(|total| *total <= cap)
        .map(|_| bytes)
        .ok_or(SolverError::ResourceLimit)
}

fn admit(args: &Args) -> Result<(TickClock, Configuration, usize, SmoothPlan), SolverError> {
    let configuration = configuration(args);
    let domain = Domain::new([args.grid; 3], [DOMAIN_LENGTH; 3], VISCOSITY)?;
    let clock = clock(args)?;
    let samples = args
        .maximum_attempts
        .checked_add(1)
        .ok_or(SolverError::SizeOverflow)?;
    let plan = SmoothPlan::from_rest(
        domain,
        clock,
        configuration,
        samples,
        ADVECTIVE_LIMIT,
        args.memory_cap,
    )?;
    Ok((clock, configuration, samples, plan))
}

fn resume(args: Args) -> ExitCode {
    let (clock, configuration, samples, plan) = match admit(&args) {
        Ok(value) => value,
        Err(error) => return refused_with_origin(error, "external_unverified"),
    };
    let imported = match import_checkpoint(&args, clock, configuration, samples, plan) {
        Ok(value) => value,
        Err(error) => return checkpoint_refused(error),
    };
    let mut run = match imported.continue_unverified(args.memory_cap) {
        Ok(value) => value,
        Err(error) => return refused_with_origin(error, "external_unverified"),
    };
    finish(args, clock, &mut run)
}

fn import_checkpoint(
    args: &Args,
    clock: TickClock,
    configuration: Configuration,
    samples: usize,
    plan: SmoothPlan,
) -> Result<nsbu_benchmarks::smooth_run::archive::ImportedSmoothRun, &'static str> {
    let maximum =
        checkpoint_preflight(plan, configuration, args.memory_cap).map_err(|_| "resource_limit")?;
    let path = args
        .checkpoint
        .as_deref()
        .expect("parser requires checkpoint");
    let bytes =
        checkpoint_io::read_checkpoint(path, maximum, args.memory_cap, plan.resources().total())?;
    let available = args
        .memory_cap
        .checked_sub(bytes.len())
        .ok_or("resource_limit")?;
    let imported =
        nsbu_benchmarks::smooth_run::archive::read(&bytes, plan.resources(), maximum, available)
            .map_err(|_| "checkpoint_invalid")?;
    if !checkpoint_io::matches_profile(&imported, configuration, clock, samples) {
        return Err("checkpoint_profile_mismatch");
    }
    Ok(imported)
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
    finish(args, clock, &mut run)
}

fn execute_checkpoint(
    args: Args,
    clock: TickClock,
    configuration: Configuration,
    samples: usize,
    path: &Path,
    maximum: usize,
) -> ExitCode {
    let domain = match Domain::new([args.grid; 3], [DOMAIN_LENGTH; 3], VISCOSITY) {
        Ok(value) => value,
        Err(error) => return refused(error),
    };
    let mut output = match checkpoint_io::bounded_buffer(maximum) {
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
    let target = args
        .checkpoint_after
        .expect("parser requires checkpoint-after");
    while run.history().controller().stopped().is_none()
        && run.history().controller().committed() < target
    {
        if let Err(error) = run.step() {
            return refused(error);
        }
    }
    if run.history().controller().committed() == target {
        let used = match nsbu_benchmarks::smooth_run::archive::write(&run, &mut output) {
            Ok(value) => value,
            Err(_) => return io_refused("checkpoint_encode_failed", "internal_from_rest"),
        };
        if let Err(error) = checkpoint_io::publish_checkpoint(path, &output[..used]) {
            return io_refused(error, "internal_from_rest");
        }
        print_checkpoint(args, clock, &run);
        return ExitCode::SUCCESS;
    }
    print_run(args, clock, &run);
    ExitCode::from(1)
}

fn finish(args: Args, clock: TickClock, run: &mut SmoothRun) -> ExitCode {
    while run.history().controller().stopped().is_none() {
        if let Err(error) = run.step() {
            return refused_with_origin(error, origin_name(run));
        }
    }
    print_run(args, clock, run);
    match run.history().controller().stopped() {
        Some(StopReason::EndpointReached) => ExitCode::SUCCESS,
        Some(_) | None => ExitCode::from(1),
    }
}
