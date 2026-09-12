use nsbu_benchmarks::{
    runtime_force::ForceSettings,
    v2_run::{Plan, Run, Settings},
};
use nsbu_solver::{
    domain::{Domain, Layout, SpectralState, TickClock},
    experiment::control::Configuration,
    integrators::{
        indicator::Tolerances,
        method::Method,
        trajectory::{RunLimits, StopReason},
    },
};
use sha2::{Digest, Sha256};
use std::{fs, process::ExitCode, time::Instant};

const CAP: usize = 34_359_738_368;
const IO_ALLOWANCE: usize = 4096;

#[derive(Debug, Clone, Copy)]
struct Arguments {
    grid: usize,
    force_grid: usize,
    step: u128,
    endpoint: u128,
    method: Method,
    workers: usize,
    deadline_seconds: u64,
}

fn main() -> ExitCode {
    match arguments().and_then(execute) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("terminal=error detail={error}");
            ExitCode::from(1)
        }
    }
}

fn arguments() -> Result<Arguments, String> {
    let values = std::env::args().skip(1).collect::<Vec<_>>();
    let [grid, force_grid, step, endpoint, method, workers, deadline] = values.as_slice() else {
        return Err(
            "usage: p10-trajectory GRID FORCE_GRID STEP ENDPOINT cm|ho WORKERS DEADLINE_SECONDS"
                .into(),
        );
    };
    let method = match method.as_str() {
        "cm" => Method::CoxMatthews,
        "ho" => Method::HochbruckOstermann,
        _ => return Err("method must be cm or ho".into()),
    };
    Ok(Arguments {
        grid: parse(grid)?,
        force_grid: parse(force_grid)?,
        step: parse(step)?,
        endpoint: parse(endpoint)?,
        method,
        workers: parse(workers)?,
        deadline_seconds: parse(deadline)?,
    })
}

fn execute(args: Arguments) -> Result<(), String> {
    let settings = settings(args)?;
    let plan = Plan::from_rest_cached(settings, CAP).map_err(debug)?;
    let snapshot_bytes = plan
        .resources()
        .domain()
        .layout()
        .half_len()
        .checked_mul(48)
        .ok_or("snapshot admission overflow")?;
    let combined = plan
        .resources()
        .total()
        .checked_add(snapshot_bytes)
        .and_then(|value| value.checked_add(IO_ALLOWANCE))
        .ok_or("combined admission overflow")?;
    println!("preflight source={} cap={CAP} run_reservation={} snapshot_bytes={snapshot_bytes} io_allowance={IO_ALLOWANCE} combined={combined} args={args:?} settings={:?} mode={:?} integration_limits={:?} observer_limits={:?}", env!("RUN_SOURCE"), plan.resources().total(), plan.settings(), plan.integration_mode(), plan.integration_limits(), plan.observer_limits());
    if combined > CAP {
        return Err(format!("combined reservation {combined} exceeds cap {CAP}"));
    }
    let mut run = Run::from_rest(plan).map_err(debug)?;
    let mut snapshot = Vec::new();
    snapshot.try_reserve_exact(snapshot_bytes).map_err(debug)?;
    snapshot.resize(snapshot_bytes, 0);
    let started = Instant::now();
    while run.history().controller().stopped().is_none() {
        if started.elapsed().as_secs() >= args.deadline_seconds {
            return stop_at_deadline(&run, &mut snapshot, args.deadline_seconds);
        }
        let step_started = Instant::now();
        let outcome = run.step().map_err(debug)?;
        println!(
            "attempt={} committed={} clock={} step_seconds={:.9} outcome={outcome:?}",
            run.history().controller().attempted(),
            run.history().controller().committed(),
            run.state().clock().elapsed(),
            step_started.elapsed().as_secs_f64(),
        );
    }
    let stop = run
        .history()
        .controller()
        .stopped()
        .ok_or("missing terminal status")?;
    if !matches!(stop, StopReason::EndpointReached) {
        capture("terminal", &run, &mut snapshot)?;
        return Err(format!("incomplete terminal={stop:?}"));
    }
    capture("accepted", &run, &mut snapshot)?;
    println!(
        "terminal=completed endpoint={} attempts={} committed={} wall_seconds={:.9} observer={:?}",
        run.state().clock().elapsed(),
        run.history().controller().attempted(),
        run.history().controller().committed(),
        started.elapsed().as_secs_f64(),
        run.observer_work()
    );
    Ok(())
}

fn settings(args: Arguments) -> Result<Settings, String> {
    let attempts = args
        .endpoint
        .checked_div(args.step)
        .ok_or("step must be positive")?;
    if attempts * args.step != args.endpoint {
        return Err("endpoint must be divisible by step".into());
    }
    let attempts = usize::try_from(attempts).map_err(debug)?;
    Ok(Settings {
        domain: Domain::new([args.grid; 3], [1.0; 3], 1.0).map_err(debug)?,
        force: ForceSettings {
            samples: Layout::new([args.force_grid; 3]).map_err(debug)?,
            workers: args.workers,
        },
        initial_clock: TickClock::from_rest(-20, 8192).map_err(debug)?,
        configuration: Configuration {
            method: args.method,
            limits: RunLimits {
                endpoint: args.endpoint,
                step_ticks: args.step,
                maximum_attempts: attempts,
            },
            tolerances: Tolerances {
                absolute: [1e-5, 1e-4],
                relative: [1e-5; 2],
            },
        },
        advective_limit: 0.3,
    })
}

fn stop_at_deadline(run: &Run, bytes: &mut [u8], seconds: u64) -> Result<(), String> {
    capture("deadline", run, bytes)?;
    Err(format!(
        "deadline reached after {seconds}s at clock {}",
        run.state().clock().elapsed()
    ))
}

fn capture(tag: &str, run: &Run, bytes: &mut [u8]) -> Result<(), String> {
    let count = encode(run.state(), bytes)?;
    let digest: [u8; 32] = Sha256::digest(&bytes[..count]).into();
    fs::create_dir_all("forensics").map_err(debug)?;
    let settings = run.plan().settings();
    let method = match settings.configuration.method {
        Method::CoxMatthews => "cm",
        Method::HochbruckOstermann => "ho",
    };
    let path = format!(
        "forensics/{tag}-n{}-m{}-{method}{}-clock{}.coeff.bin",
        settings.domain.layout().dimensions()[0],
        settings.force.samples.dimensions()[0],
        settings.configuration.limits.step_ticks,
        run.state().clock().elapsed()
    );
    fs::write(&path, &bytes[..count]).map_err(debug)?;
    println!("forensic tag={tag} path={path} bytes={count} sha256={digest:02x?}");
    Ok(())
}

fn encode(state: &SpectralState, bytes: &mut [u8]) -> Result<usize, String> {
    let need = state
        .plan()
        .domain()
        .layout()
        .half_len()
        .checked_mul(48)
        .ok_or("snapshot size overflow")?;
    if bytes.len() < need {
        return Err("snapshot buffer mismatch".into());
    }
    let mut at = 0;
    for axis in 0..3 {
        for coefficient in state.component(axis).map_err(debug)? {
            bytes[at..at + 8].copy_from_slice(&coefficient.re.to_bits().to_le_bytes());
            bytes[at + 8..at + 16].copy_from_slice(&coefficient.im.to_bits().to_le_bytes());
            at += 16;
        }
    }
    Ok(at)
}

fn parse<T: std::str::FromStr>(value: &str) -> Result<T, String> {
    value
        .parse()
        .map_err(|_| format!("invalid argument {value}"))
}

fn debug(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}
