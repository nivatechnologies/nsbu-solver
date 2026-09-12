//! Ignored preflight/run harness for the cached-M48 diagnostic pilot.
//! This is a diagnostic experiment only; it does not write or read v1 checkpoints.
use nsbu_benchmarks::{
    runtime_force::{ForceSettings, IntegrationMode},
    v2_experiment::{
        diagnostic::{DiagnosticConsumerWork, DiagnosticDriver, DiagnosticPlan, DiagnosticSettings},
        probes::ProbePlan,
        FamilyPlan, FamilySettings,
    },
    CASE_SHA256,
};
use nsbu_solver::{
    domain::{Layout, SpectralState, TickClock},
    integrators::indicator::Tolerances,
    verification::times::TestedTimes,
};
use sha2::{Digest, Sha256};
use std::{fs, time::Instant};

const CAP: usize = 4 * 1024 * 1024 * 1024;
const IO_SCRATCH: usize = 4096;
const COEFFICIENT_WORD_BYTES: usize = 16;

fn main() {
    if let Err(error) = execute() {
        eprintln!("pilot_failure={error}");
        std::process::exit(2);
    }
}

fn execute() -> Result<(), String> {
    let mode = std::env::args().nth(1).unwrap_or_else(|| "--dry-run".into());
    if !matches!(mode.as_str(), "--dry-run" | "--walkthrough" | "--full") {
        return Err(format!("unsupported mode {mode}"));
    }
    let endpoint = if mode == "--walkthrough" { 128 } else { 4096 };
    let accepted = clocks(if endpoint == 128 { &[0, 64, 128] } else { &[0, 2048, 4096] });
    let probes = clocks(if endpoint == 128 { &[0, 63, 64, 127, 128] } else { &[0, 2047, 2048, 4095, 4096] });
    let residual = [probes[1], probes[3]];
    let family = FamilyPlan::new_cached(settings(endpoint), TestedTimes::new(&accepted, accepted.len()).map_err(debug)?, CAP).map_err(debug)?;
    if family.integration_mode() != IntegrationMode::AttemptCached {
        return Err("cached profile was not admitted".into());
    }
    let probe = ProbePlan::new(family, TestedTimes::new(&probes, probes.len()).map_err(debug)?, probes.len(), CAP).map_err(debug)?;
    let diagnostic = DiagnosticPlan::new(family, probe, &residual, policy(), CAP).map_err(debug)?;
    let forensic_bytes = largest_state_bytes(family)?;
    let harness_bytes = forensic_bytes.checked_add(IO_SCRATCH).ok_or("harness overflow")?;
    let combined = diagnostic.bounds().joint_storage_bytes.checked_add(harness_bytes).ok_or("combined overflow")?;
    println!("source={} case_sha256={} mode={} endpoint={} cap_bytes={} combined_bytes={} diagnostic_bytes={} forensic_bytes={} io_scratch_bytes={}", env!("PILOT_SOURCE"), CASE_SHA256, mode, endpoint, CAP, combined, diagnostic.bounds().joint_storage_bytes, forensic_bytes, IO_SCRATCH);
    println!("cached_trajectory_settings={:?} integration_mode={:?} diagnostic_settings={:?} diagnostic_bounds={:?}", family.settings(), family.integration_mode(), diagnostic.diagnostic_settings(), diagnostic.bounds());
    println!("accepted_elapsed={:?} probe_elapsed={:?} residual_elapsed={:?} worker_profile=32 integration_force=M48 observer_doubled_force=M96 physical_reference=24 pressure=48 checkpoint_v1=unsupported", accepted.iter().map(|clock| clock.elapsed()).collect::<Vec<_>>(), probes.iter().map(|clock| clock.elapsed()).collect::<Vec<_>>(), residual.iter().map(|clock| clock.elapsed()).collect::<Vec<_>>());
    if combined > CAP { return Err(format!("combined reservation {combined} exceeds cap {CAP}")); }
    if mode == "--dry-run" { println!("terminal=dry-run-complete"); return Ok(()); }
    run(diagnostic, forensic_bytes)
}

fn run(plan: DiagnosticPlan<'_>, forensic_bytes: usize) -> Result<(), String> {
    let started = Instant::now();
    let mut driver = DiagnosticDriver::new(plan).map_err(debug)?;
    let mut forensic = vec![0_u8; forensic_bytes];
    let mut events = 0usize;
    loop {
        let attempted = driver.next_time();
        let before = driver.consumer_work();
        match driver.advance() {
            Ok(Some(event)) => {
                events += 1;
                emit_branch_forensics(&driver, event.clock(), "event", &mut forensic)?;
                println!("event={} elapsed={} wall_seconds={:.6} diagnostic_raw={:?}", events, event.clock().elapsed(), started.elapsed().as_secs_f64(), event);
            }
            Ok(None) => {
                if events != 5 { return Err(format!("incomplete terminal events={events}")); }
                println!("terminal=complete events={} wall_seconds={:.6} charged={:?} consumers={:?}", events, started.elapsed().as_secs_f64(), driver.charged_work(), driver.consumer_work());
                return Ok(());
            }
            Err(error) => {
                let clock = attempted.ok_or("failure without attempted clock")?;
                let after = driver.consumer_work();
                let files = emit_all_available_forensics(&driver, clock, "failure", &mut forensic)?;
                println!("terminal=diagnostic-error events={} attempted_elapsed={} wall_seconds={:.6} inferred_phase={} forensic_files={} error={:?} charged={:?} consumers_before={:?} consumers_after={:?}", events, clock.elapsed(), started.elapsed().as_secs_f64(), failure_phase(before, after), files, error, driver.charged_work(), before, after);
                return Err(format!("diagnostic error at {}: {error:?}", clock.elapsed()));
            }
        }
    }
}

fn emit_branch_forensics(driver: &DiagnosticDriver<'_>, clock: TickClock, tag: &str, bytes: &mut [u8]) -> Result<(), String> {
    for index in 0..6 {
        let ordinary = driver.ordinary().branch(index).ok_or("missing ordinary branch")?;
        if ordinary.state().clock() == clock { emit_state("ordinary", index, tag, ordinary.state(), ordinary.work(), ordinary.cache_work(), bytes)?; }
        let probe = driver.probes().branch(index).ok_or("missing probe branch")?;
        if probe.state().clock() == clock { emit_state("probe", index, tag, probe.state(), probe.work(), probe.cache_work(), bytes)?; }
    }
    Ok(())
}

fn emit_all_available_forensics(driver: &DiagnosticDriver<'_>, clock: TickClock, tag: &str, bytes: &mut [u8]) -> Result<usize, String> {
    let mut count = 0;
    for index in 0..6 {
        let ordinary = driver.ordinary().branch(index).ok_or("missing ordinary branch")?;
        emit_state("ordinary", index, tag, ordinary.state(), ordinary.work(), ordinary.cache_work(), bytes)?;
        let probe = driver.probes().branch(index).ok_or("missing probe branch")?;
        emit_state("probe", index, tag, probe.state(), probe.work(), probe.cache_work(), bytes)?;
        count += 2;
    }
    println!("failure_forensics_requested_clock={}", clock.elapsed());
    Ok(count)
}

fn emit_state<T: std::fmt::Debug>(owner: &str, index: usize, tag: &str, state: &SpectralState, ledger: &[T], cache: Option<nsbu_benchmarks::runtime_force::AttemptCacheWork>, bytes: &mut [u8]) -> Result<(), String> {
    let used = encode_coefficients(state, bytes)?;
    let digest: [u8; 32] = Sha256::digest(&bytes[..used]).into();
    let path = format!("forensics/{tag}-{owner}-branch{index}-clock{}.coeff.bin", state.clock().elapsed());
    fs::create_dir_all("forensics").map_err(debug)?;
    fs::write(&path, &bytes[..used]).map_err(debug)?;
    println!("coeff_forensics owner={} branch={} path={} bytes={} sha256={:02x?} clock={} ledger={:?} cache={:?}", owner, index, path, used, digest, state.clock().elapsed(), ledger, cache);
    Ok(())
}

fn encode_coefficients(state: &SpectralState, output: &mut [u8]) -> Result<usize, String> {
    let words = state.plan().domain().layout().half_len().checked_mul(3).ok_or("coefficient count overflow")?;
    let need = words.checked_mul(COEFFICIENT_WORD_BYTES).ok_or("coefficient bytes overflow")?;
    if output.len() < need { return Err("forensic buffer too small".into()); }
    let mut p = 0;
    for axis in 0..3 { for value in state.component(axis).map_err(debug)? {
        output[p..p+8].copy_from_slice(&value.re.to_bits().to_le_bytes());
        output[p+8..p+16].copy_from_slice(&value.im.to_bits().to_le_bytes());
        p += 16;
    }}
    Ok(p)
}

fn largest_state_bytes(family: FamilyPlan<'_>) -> Result<usize, String> {
    let mut maximum = 0;
    for index in 0..6 {
        let plan = family.branch_plan(index).ok_or("missing plan")?;
        let bytes = plan.resources().domain().layout().half_len()
            .checked_mul(3).and_then(|n| n.checked_mul(COEFFICIENT_WORD_BYTES))
            .ok_or("forensic overflow")?;
        maximum = maximum.max(bytes);
    }
    Ok(maximum)
}

fn failure_phase(before: DiagnosticConsumerWork, after: DiagnosticConsumerWork) -> &'static str {
    if after.binding.attempts > before.binding.attempts { "binding" } else if after.regional.attempts > before.regional.attempts { "regional" } else if after.reference.attempts > before.reference.attempts { "reference" } else if after.pressure.attempts > before.pressure.attempts { "pressure" } else if after.physical.attempts > before.physical.attempts { "physical" } else if after.residual.attempts > before.residual.attempts { "residual" } else if after.probes.attempts > before.probes.attempts { "probes" } else { "ordinary" }
}

fn settings(endpoint: u128) -> FamilySettings { FamilySettings { grids: [12,16,24], steps: [64,32,16], force: ForceSettings { samples: Layout::new([48;3]).unwrap(), workers: 32 }, endpoint, tolerances: Tolerances { absolute: [1e-5,1e-4], relative: [0.0;2] }, advective_limit: 0.3 } }
fn policy() -> DiagnosticSettings { DiagnosticSettings { physical_samples: Layout::new([24;3]).unwrap(), pressure_samples: Layout::new([48;3]).unwrap(), reference_samples: Layout::new([24;3]).unwrap(), physical_floors: [1e-8,1e-7,1e-6,1e-7], pressure_floors: [1e-8,1e-7], reference_floors: [1e-8,1e-7,1e-6,1e-7], regional_root_budget: 128 } }
fn clocks(values: &[u128]) -> Vec<TickClock> { values.iter().map(|&elapsed| TickClock::restore(-20,8192,elapsed,8192-elapsed).unwrap()).collect() }
fn debug(error: impl std::fmt::Debug) -> String { format!("{error:?}") }
