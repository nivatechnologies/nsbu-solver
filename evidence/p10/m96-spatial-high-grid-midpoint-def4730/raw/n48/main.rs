use nsbu_benchmarks::{
    runtime_force::ForceSettings,
    v2_run::{Plan, Run, Settings},
};
use nsbu_solver::{
    domain::{Domain, Layout, SpectralState, TickClock},
    experiment::control::Configuration,
    integrators::{indicator::Tolerances, method::Method, trajectory::{RunLimits, StopReason}},
};
use sha2::{Digest, Sha256};
use std::{fs, process::ExitCode, time::{Duration, Instant}};

const CAP: usize = 1 << 30;
const IO_ALLOWANCE: usize = 4096;
const SNAPSHOT_BYTES: usize = 2_764_800;
const ENDPOINT: u128 = 2048;
const GRID: usize = 48;

fn main() -> ExitCode {
    let mode = match std::env::args().skip(1).collect::<Vec<_>>().as_slice() {
        [] => None,
        [flag] if flag == "--dry-run" => None,
        [run, deadline, seconds] if run == "--run" && deadline == "--deadline-seconds" => match seconds.parse::<u64>() {
            Ok(value) => Some(Duration::from_secs(value)),
            Err(_) => { eprintln!("deadline seconds must be an unsigned integer"); return ExitCode::from(2); }
        },
        _ => {
            eprintln!("usage: p10-m96-next-nGRID-forensics [--dry-run|--run --deadline-seconds SECONDS]");
            return ExitCode::from(2);
        }
    };
    match execute(mode) {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => { eprintln!("terminal=error detail={error}"); ExitCode::from(1) }
    }
}

fn execute(deadline: Option<Duration>) -> Result<(), String> {
    let plan = Plan::from_rest_cached(settings(), CAP).map_err(debug)?;
    let snapshot_expected = plan.resources().domain().layout().half_len()
        .checked_mul(3).and_then(|value| value.checked_mul(16))
        .ok_or("snapshot admission overflow")?;
    if snapshot_expected != SNAPSHOT_BYTES {
        return Err(format!("snapshot admission mismatch declared={SNAPSHOT_BYTES} expected={snapshot_expected}"));
    }
    let reservation = plan.resources().total();
    let combined = reservation.checked_add(SNAPSHOT_BYTES).and_then(|n| n.checked_add(IO_ALLOWANCE)).ok_or("combined reservation overflow")?;
    println!("preflight source={} cap={} run_reservation={} snapshot_bytes={} io_allowance={} combined={} settings={:?} mode={:?} integration_limits={:?} observer_limits={:?}", env!("RUN_SOURCE"), CAP, reservation, SNAPSHOT_BYTES, IO_ALLOWANCE, combined, plan.settings(), plan.integration_mode(), plan.integration_limits(), plan.observer_limits());
    if combined > CAP { return Err(format!("combined reservation {combined} exceeds cap {CAP}")); }
    let Some(deadline) = deadline else { println!("terminal=dry-run-complete"); return Ok(()); };
    let started = Instant::now();
    let mut run = Run::from_rest(plan).map_err(debug)?;
    let mut buffer = Vec::new();
    buffer.try_reserve_exact(SNAPSHOT_BYTES).map_err(debug)?;
    buffer.resize(SNAPSHOT_BYTES, 0);
    let mut captured_2048 = false;
    let mut captured_4096 = false;
    while run.history().controller().stopped().is_none() {
        if started.elapsed() >= deadline {
            capture("deadline", &run, &mut buffer)?;
            println!("terminal=deadline clock={} attempts={} committed={} deadline_seconds={}", run.state().clock().elapsed(), run.history().controller().attempted(), run.history().controller().committed(), deadline.as_secs());
            return Err("internal monotonic deadline reached".into());
        }
        let outcome = match run.step() {
            Ok(value) => value,
            Err(error) => {
                capture("terminal", &run, &mut buffer)?;
                return Err(debug(error));
            }
        };
        let elapsed = run.state().clock().elapsed();
        if elapsed == 2048 && !captured_2048 { capture("accepted", &run, &mut buffer)?; captured_2048 = true; }
        if elapsed == ENDPOINT && !captured_4096 { capture("accepted", &run, &mut buffer)?; captured_4096 = true; }
        println!("attempt={} committed={} clock={} outcome={outcome:?} ledger={:?} cache={:?}", run.history().controller().attempted(), run.history().controller().committed(), elapsed, run.work(), run.cache_work());
    }
    let stop = run.history().controller().stopped().ok_or("missing terminal status")?;
    if !matches!(stop, StopReason::EndpointReached) {
        capture("terminal", &run, &mut buffer)?;
        return Err(format!("incomplete terminal={stop:?} clock={} attempts={} committed={}", run.state().clock().elapsed(), run.history().controller().attempted(), run.history().controller().committed()));
    }
    if !captured_4096 { return Err("endpoint reached without endpoint forensic snapshot".into()); }
    println!("terminal=completed endpoint={} attempts={} committed={} work={:?} observer={:?} cache={:?}", run.state().clock().elapsed(), run.history().controller().attempted(), run.history().controller().committed(), run.work(), run.observer_work(), run.cache_work());
    Ok(())
}

fn settings() -> Settings {
    Settings {
        domain: Domain::new([GRID; 3], [1.0; 3], 1.0).unwrap(),
        force: ForceSettings { samples: Layout::new([96; 3]).unwrap(), workers: 32 },
        initial_clock: TickClock::from_rest(-20, 8192).unwrap(),
        configuration: Configuration {
            method: Method::CoxMatthews,
            limits: RunLimits { endpoint: ENDPOINT, step_ticks: 16, maximum_attempts: 128 },
            tolerances: Tolerances { absolute: [1e-5, 1e-4], relative: [1e-5; 2] },
        },
        advective_limit: 0.3,
    }
}

fn capture(tag: &str, run: &Run, bytes: &mut [u8]) -> Result<(), String> {
    let state = run.state();
    let count = encode(state, bytes)?;
    let digest: [u8; 32] = Sha256::digest(&bytes[..count]).into();
    fs::create_dir_all("forensics").map_err(debug)?;
    let path = format!("forensics/{tag}-n48-m96-cm16-clock{}.coeff.bin", state.clock().elapsed());
    fs::write(&path, &bytes[..count]).map_err(debug)?;
    println!("forensic tag={tag} path={path} bytes={count} sha256={digest:02x?} clock={:?} attempts={} committed={} ledger={:?} cache={:?}", state.clock(), run.history().controller().attempted(), run.history().controller().committed(), run.work(), run.cache_work());
    Ok(())
}

fn encode(state: &SpectralState, bytes: &mut [u8]) -> Result<usize, String> {
    let need = state.plan().domain().layout().half_len().checked_mul(3).and_then(|n| n.checked_mul(16)).ok_or("snapshot size overflow")?;
    if need != SNAPSHOT_BYTES || bytes.len() < need { return Err("snapshot buffer mismatch".into()); }
    let mut at = 0;
    for axis in 0..3 { for coefficient in state.component(axis).map_err(debug)? {
        bytes[at..at + 8].copy_from_slice(&coefficient.re.to_bits().to_le_bytes());
        bytes[at + 8..at + 16].copy_from_slice(&coefficient.im.to_bits().to_le_bytes());
        at += 16;
    }}
    Ok(at)
}

fn debug(error: impl std::fmt::Debug) -> String { format!("{error:?}") }
