//! JSON output for the bounded smooth diagnostic and checkpoint commands.
use crate::arguments::{Args, MethodName};
use nsbu_benchmarks::{
    smooth_observer::{BalanceObserver, BalanceObserverLimits, BalanceObserverWork},
    smooth_run::{IntegrationWork, Origin, SmoothPlan, SmoothRun},
};
use nsbu_solver::{domain::TickClock, integrators::trajectory::StopReason, SolverError};
use std::process::ExitCode;

pub(crate) fn print_plan(args: Args, clock: TickClock, plan: SmoothPlan, checkpoint_bytes: usize) {
    let resources = plan.resources();
    let observer = match BalanceObserver::limits(resources.domain(), plan.observer_samples()) {
        Ok(value) => value,
        Err(error) => {
            let _ = refused(error);
            return;
        }
    };
    let checkpoint = if checkpoint_bytes == 0 {
        String::new()
    } else {
        format!(",\"checkpoint\":{{\"maximum_bytes\":\"{checkpoint_bytes}\"}}")
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
            "\"bounded_observer\":{}{} }}"
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
        checkpoint,
    );
}

pub(crate) fn print_run(args: Args, clock: TickClock, run: &SmoothRun) {
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
            "\"qualification_status\":\"unqualified\",\"origin_status\":\"{}\",",
            "\"method\":\"{}\",\"stop_reason\":\"{}\",\"clock\":{},",
            "\"tolerances\":{},\"attempts\":{{\"started\":\"{}\",\"committed\":\"{}\"}},",
            "\"integration\":{{\"rhs_calls\":\"{}\",\"work_units\":\"{}\",",
            "\"scalar_transforms\":\"{}\"}},\"diagnostics\":{} }}"
        ),
        status,
        origin_name(run),
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

pub(crate) fn print_checkpoint(args: Args, clock: TickClock, run: &SmoothRun) {
    let controller = run.history().controller();
    println!(
        concat!(
            "{{\"status\":\"checkpoint_saved\",\"profile\":{{\"name\":\"CyclicSine\",",
            "\"lengths\":[1.0,1.0,1.0],\"viscosity\":0.3,\"pde_qualified\":false}},",
            "\"qualification_status\":\"unqualified\",\"origin_status\":\"{}\",",
            "\"method\":\"{}\",\"clock\":{},\"attempts\":{{\"started\":\"{}\",\"committed\":\"{}\"}}}}"
        ),
        origin_name(run), method_name(args.method), clock_json(args, clock, &controller.clock().elapsed().to_string()),
        controller.attempted(), controller.committed(),
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

pub(crate) fn refused(error: SolverError) -> ExitCode {
    refused_with_origin(error, "internal_from_rest")
}

pub(crate) fn refused_with_origin(error: SolverError, origin: &str) -> ExitCode {
    println!(
        "{{\"status\":\"refused\",\"qualification_status\":\"unqualified\",\"origin_status\":\"{}\",\"profile\":{{\"name\":\"CyclicSine\",\"pde_qualified\":false}},\"error\":\"{}\"}}",
        origin, error_name(error)
    );
    ExitCode::from(1)
}

pub(crate) fn checkpoint_refused(error: &str) -> ExitCode {
    io_refused(error, "external_unverified")
}

pub(crate) fn io_refused(error: &str, origin: &str) -> ExitCode {
    let status = if error == "checkpoint_published_durability_unconfirmed" {
        "checkpoint_published_durability_unconfirmed"
    } else {
        "refused"
    };
    println!("{{\"status\":\"{}\",\"qualification_status\":\"unqualified\",\"origin_status\":\"{}\",\"profile\":{{\"name\":\"CyclicSine\",\"pde_qualified\":false}},\"error\":\"{}\"}}", status, origin, error);
    ExitCode::from(1)
}

pub(crate) fn origin_name(run: &SmoothRun) -> &'static str {
    match run.origin() {
        Origin::InternalFromRest => "internal_from_rest",
        Origin::ExternalUnverified => "external_unverified",
    }
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
