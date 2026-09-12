//! Admission, execution and file boundaries for the exact-v2 CLI profile.
use crate::{
    arguments::{Args, MethodName},
    checkpoint_io, v2_report,
};
use nsbu_benchmarks::{
    runtime_force::ForceSettings,
    v2_run::{archive, Plan, Run, Settings},
};
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
    experiment::control::Configuration,
    integrators::{
        indicator::Tolerances,
        method::Method,
        trajectory::{RunLimits, StopReason},
    },
    SolverError,
};
use std::process::ExitCode;

pub(crate) fn execute(args: Args, resume: bool) -> ExitCode {
    let origin = if resume {
        "external_unverified"
    } else {
        "internal_from_rest"
    };
    if args.cache_force && (resume || args.checkpoint.is_some() || args.checkpoint_after.is_some())
    {
        return v2_report::cache_checkpoint_refused(origin);
    }
    let plan = match admit(&args) {
        Ok(plan) => plan,
        Err(error) => return v2_report::refused(error, origin),
    };
    let (maximum, storage) = match checkpoint_budget(&args, plan, resume) {
        Ok(value) => value,
        Err(error) => return v2_report::refused(error, origin),
    };
    if args.dry_run {
        if args.cache_force {
            v2_report::cached_plan(&args, plan, maximum);
        } else {
            v2_report::plan(&args, plan, maximum);
        }
        return ExitCode::SUCCESS;
    }
    if resume {
        return resume_run(&args, plan, maximum, storage);
    }
    let mut run = match Run::from_rest(plan) {
        Ok(run) => run,
        Err(error) => return v2_report::refused(error, origin),
    };
    if args.checkpoint.is_some() {
        checkpoint_run(&args, &mut run, maximum)
    } else {
        finish(&args, &mut run)
    }
}

fn admit(args: &Args) -> Result<Plan, SolverError> {
    let shift = (-7i32)
        .checked_sub(args.tick_exponent)
        .and_then(|n| u32::try_from(n).ok())
        .ok_or(SolverError::ClockCapacityExceeded)?;
    let target = 1u128
        .checked_shl(shift)
        .ok_or(SolverError::ClockCapacityExceeded)?;
    let settings = Settings {
        domain: Domain::new([args.grid; 3], [1.0; 3], 1.0)?,
        force: ForceSettings {
            samples: Layout::new([args.force_grid.unwrap_or(args.grid); 3])?,
            workers: args.workers,
        },
        initial_clock: TickClock::from_rest(args.tick_exponent, target)?,
        configuration: Configuration {
            method: match args.method {
                MethodName::CoxMatthews => Method::CoxMatthews,
                MethodName::HochbruckOstermann => Method::HochbruckOstermann,
            },
            limits: RunLimits {
                endpoint: args.endpoint_ticks,
                step_ticks: args.step_ticks,
                maximum_attempts: args.maximum_attempts,
            },
            tolerances: Tolerances {
                absolute: [1e-5, 1e-4],
                relative: [1e-5; 2],
            },
        },
        advective_limit: 0.3,
    };
    let plan = if args.cache_force {
        Plan::from_rest_cached(settings, args.memory_cap)?
    } else {
        Plan::from_rest(settings, args.memory_cap)?
    };
    if args
        .checkpoint_after
        .is_some_and(|n| n as u128 > args.endpoint_ticks / args.step_ticks)
    {
        return Err(SolverError::InvalidStep);
    }
    Ok(plan)
}

fn checkpoint_budget(args: &Args, plan: Plan, resume: bool) -> Result<(usize, usize), SolverError> {
    if args.checkpoint.is_none() {
        return Ok((0, plan.resources().total()));
    }
    let maximum = archive::maximum_encoded_len(plan).map_err(|_| SolverError::ResourceLimit)?;
    let storage = if resume {
        archive::read_reservation(plan, plan.settings().configuration.limits.maximum_attempts)
            .map_err(|_| SolverError::ResourceLimit)?
    } else {
        plan.resources().total()
    };
    if storage
        .checked_add(maximum)
        .filter(|n| *n <= args.memory_cap)
        .is_none()
    {
        return Err(SolverError::ResourceLimit);
    }
    Ok((maximum, storage))
}

fn finish(args: &Args, run: &mut Run) -> ExitCode {
    while run.history().controller().stopped().is_none() {
        if let Err(error) = run.step() {
            return v2_report::refused(error, v2_report::origin(run));
        }
    }
    if args.cache_force {
        v2_report::cached_run(args, run);
    } else {
        v2_report::run(args, run);
    }
    if matches!(
        run.history().controller().stopped(),
        Some(StopReason::EndpointReached)
    ) {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(1)
    }
}

fn checkpoint_run(args: &Args, run: &mut Run, maximum: usize) -> ExitCode {
    // Parsing admits these two options together, and admission checks the requested step count.
    let target = args
        .checkpoint_after
        .expect("checkpoint options admitted together");
    while run.history().controller().stopped().is_none()
        && run.history().controller().committed() < target
    {
        if let Err(error) = run.step() {
            return v2_report::refused(error, v2_report::origin(run));
        }
    }
    if run.history().controller().committed() != target {
        v2_report::run(args, run);
        return ExitCode::from(1);
    }
    if let Err(error) = publish(args, run, maximum) {
        return v2_report::io_refused(error, v2_report::origin(run));
    }
    v2_report::checkpoint(args, run);
    ExitCode::SUCCESS
}

fn publish(args: &Args, run: &Run, maximum: usize) -> Result<(), &'static str> {
    let path = args
        .checkpoint
        .as_deref()
        .expect("checkpoint path admitted");
    let mut output =
        checkpoint_io::bounded_buffer(maximum).map_err(|_| "checkpoint_allocation_failed")?;
    let count = archive::write(run, &mut output).map_err(|_| "checkpoint_encode_failed")?;
    checkpoint_io::publish_checkpoint(path, &output[..count])
}

fn resume_run(args: &Args, plan: Plan, maximum: usize, storage: usize) -> ExitCode {
    let path = args.checkpoint.as_deref().expect("resume path admitted");
    let bytes = match checkpoint_io::read_checkpoint(path, maximum, args.memory_cap, storage) {
        Ok(bytes) => bytes,
        Err(error) => return v2_report::io_refused(error, "external_unverified"),
    };
    let imported = match archive::read(&bytes, plan, maximum, args.memory_cap - bytes.len()) {
        Ok(run) => run,
        Err(_) => return v2_report::io_refused("checkpoint_invalid", "external_unverified"),
    };
    drop(bytes);
    let mut run = match imported.continue_unverified(args.memory_cap) {
        Ok(run) => run,
        Err(error) => return v2_report::refused(error, "external_unverified"),
    };
    finish(args, &mut run)
}
