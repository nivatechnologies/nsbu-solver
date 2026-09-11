//! JSON summaries of the actual v2 trajectory, its measured balances and charged work.
use crate::arguments::{Args, MethodName};
use nsbu_benchmarks::{
    smooth_observer::BalanceObserverWork,
    v2_run::{Origin, Plan, Run},
};
use nsbu_solver::{
    diagnostics::balances::BalanceSample, domain::TickClock, experiment::control::Outcome,
    integrators::trajectory::StopReason, SolverError,
};
use std::process::ExitCode;

pub(crate) fn plan(args: &Args, plan: Plan, checkpoint: usize) {
    let integration = plan.integration_limits().map(|n| n as u128);
    let observer = plan.observer_limits();
    println!(concat!("{{\"status\":\"dry_run\",{},\"clock\":{},",
        "\"work_limits\":{{\"integration\":{},\"observation\":{}}},",
        "\"memory\":{{\"total_bytes\":\"{}\",\"checkpoint_buffer_bytes\":\"{}\",\"cap_bytes\":\"{}\"}}}}"),
        profile(args, "internal_from_rest"), clock(args, plan.settings().initial_clock),
        integration_json(integration), observation_json(BalanceObserverWork { samples: observer.samples,
            work_units: observer.work_units, scalar_transforms: observer.scalar_transforms }),
        plan.resources().total(), checkpoint, args.memory_cap);
}

pub(crate) fn run(args: &Args, run: &Run) {
    let (status, reason) = match run.history().controller().stopped() {
        Some(StopReason::EndpointReached) => ("completed", "endpoint_reached"),
        Some(StopReason::LocalErrorRejected) => ("failed", "local_error_rejected"),
        Some(StopReason::Refused(error)) => ("failed", crate::report::error_name(error)),
        None => ("failed", "not_stopped"),
    };
    snapshot(args, run, status, reason);
}

pub(crate) fn checkpoint(args: &Args, run: &Run) {
    snapshot(args, run, "checkpoint_saved", "checkpoint_requested");
}

fn snapshot(args: &Args, run: &Run, status: &str, reason: &str) {
    let controller = run.history().controller();
    let counts = outcomes(run);
    println!(concat!("{{\"status\":\"{}\",{},\"stop_reason\":\"{}\",\"clock\":{},",
        "\"attempts\":{{\"started\":\"{}\",\"committed\":\"{}\",\"rejected\":\"{}\",\"refused\":\"{}\"}},",
        "\"actual_charged_work\":{{\"integration\":{},\"observation\":{}}},\"diagnostics\":{}}}"),
        status, profile(args, origin(run)), reason, clock(args, controller.clock()),
        controller.attempted(), counts[0], counts[1], counts[2],
        integration_json(total_work(run)), observation_json(run.observer_work()), diagnostics(run));
}

fn profile(args: &Args, origin: &str) -> String {
    let method = match args.method {
        MethodName::CoxMatthews => "cm",
        MethodName::HochbruckOstermann => "ho",
    };
    format!(concat!("\"profile\":{{\"name\":\"similarity-mms-v2\",\"sha256\":\"{}\",",
        "\"grid\":{},\"force_grid\":{},\"workers\":{},\"lengths\":[1.0,1.0,1.0],\"viscosity\":1.0,\"pde_qualified\":false}},",
        "\"qualification_status\":\"unqualified\",\"accepted_pde_windows\":0,\"origin_status\":\"{}\",\"method\":\"{}\",",
        "\"tolerances\":{{\"absolute\":[1e-5,1e-4],\"relative\":[1e-5,1e-5],\"advective_guard\":0.3}}"),
        nsbu_benchmarks::CASE_SHA256, args.grid, args.force_grid.unwrap_or(args.grid), args.workers, origin, method)
}

fn clock(args: &Args, clock: TickClock) -> String {
    format!(concat!("{{\"tick_exponent\":{},\"requested_endpoint_ticks\":\"{}\",",
        "\"actual_elapsed_ticks\":\"{}\",\"remaining_ticks\":\"{}\",\"step_ticks\":\"{}\",\"clock_target_ticks\":\"{}\"}}"),
        args.tick_exponent, args.endpoint_ticks, clock.elapsed(), clock.remaining(), args.step_ticks, clock.target())
}

fn outcomes(run: &Run) -> [usize; 3] {
    let mut counts = [0; 3];
    for record in run.history().records() {
        match record.outcome {
            Outcome::Committed(_) => counts[0] += 1,
            Outcome::Rejected(_) => counts[1] += 1,
            Outcome::Refused { .. } => counts[2] += 1,
        }
    }
    counts
}

fn total_work(run: &Run) -> [u128; 3] {
    let mut total = [0; 3];
    // At most usize::MAX entries with usize charges: each total fits in u128.
    for entry in run.work() {
        for (sum, value) in total.iter_mut().zip(entry.integration()) {
            *sum += value as u128;
        }
    }
    total
}

fn integration_json(work: [u128; 3]) -> String {
    format!("[\"{}\",\"{}\",\"{}\"]", work[0], work[1], work[2])
}
fn observation_json(work: BalanceObserverWork) -> String {
    format!(
        "{{\"samples\":\"{}\",\"work_units\":\"{}\",\"scalar_transforms\":\"{}\"}}",
        work.samples, work.work_units, work.scalar_transforms
    )
}

fn diagnostics(run: &Run) -> String {
    let sample = run
        .history()
        .records()
        .iter()
        .rev()
        .find_map(|record| record.sample);
    let sample_json = sample.map(sample_json).unwrap_or_else(|| "null".into());
    let balance = run.history().balance();
    // A pending Simpson midpoint is retained; do not report an incomplete pair as an integral.
    let integral = match balance.integral() {
        Ok(value) => format!("{{\"energy_rhs\":{},\"enstrophy_rhs\":{},\"energy_defect\":{},\"enstrophy_defect\":{}}}",
            value.energy_rhs, value.enstrophy_rhs, value.energy_defect, value.enstrophy_defect),
        Err(_) => "null".into(),
    };
    format!("{{\"samples_including_rest\":\"{}\",\"pending_midpoint\":{},\"last_accepted_sample\":{},\"balance_integral\":{}}}",
        balance.samples(), balance.has_pending_midpoint(), sample_json, integral)
}

fn sample_json(sample: BalanceSample) -> String {
    format!(
        concat!(
            "{{\"energy\":{},\"enstrophy\":{},\"energy_dissipation\":{},\"forcing_work\":{},",
            "\"stretching\":{},\"enstrophy_dissipation\":{},\"vorticity_forcing\":{},",
            "\"norms\":{{\"l2\":{},\"h1\":{},\"vorticity_l2\":{},\"divergence_l2\":{}}}}}"
        ),
        sample.energy,
        sample.enstrophy,
        sample.energy_dissipation,
        sample.forcing_work,
        sample.stretching,
        sample.enstrophy_dissipation,
        sample.vorticity_forcing,
        sample.norms.l2,
        sample.norms.h1,
        sample.norms.vorticity_l2,
        sample.norms.divergence_l2
    )
}

pub(crate) fn origin(run: &Run) -> &'static str {
    match run.origin() {
        Origin::InternalFromRest => "internal_from_rest",
        Origin::ExternalUnverified => "external_unverified",
    }
}

pub(crate) fn refused(error: SolverError, origin: &str) -> ExitCode {
    io_refused(crate::report::error_name(error), origin)
}

pub(crate) fn io_refused(error: &str, origin: &str) -> ExitCode {
    let status = if error == "checkpoint_published_durability_unconfirmed" {
        error
    } else {
        "refused"
    };
    // Both labels come from closed internal sets, never from unescaped user file names.
    println!(concat!("{{\"status\":\"{}\",\"profile\":{{\"name\":\"similarity-mms-v2\",\"pde_qualified\":false}},",
        "\"qualification_status\":\"unqualified\",\"accepted_pde_windows\":0,\"origin_status\":\"{}\",\"error\":\"{}\"}}"),
        status, origin, error);
    ExitCode::from(1)
}
