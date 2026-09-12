use nsbu_benchmarks::{
    runtime_force::ForceSettings,
    v2_experiment::{
        diagnostic::{
            DiagnosticConsumerWork, DiagnosticDriver, DiagnosticPlan, DiagnosticSettings,
        },
        probes::ProbePlan,
        FamilyPlan, FamilySettings,
    },
    v2_run::{archive, Plan as RunPlan, Run, Settings as RunSettings},
    CASE_SHA256,
};
use nsbu_solver::{
    diagnostics::comparison::{BandComparison, ComparisonPlan},
    domain::{Domain, Layout, SpectralState, TickClock},
    experiment::control::{Configuration, Outcome},
    integrators::{indicator::Tolerances, method::Method, trajectory::RunLimits},
    verification::times::TestedTimes,
};
use sha2::{Digest, Sha256};
use std::{fs, io::Write, path::Path, time::Instant};

const CAP: usize = 4 * 1024 * 1024 * 1024;
const IO_SCRATCH: usize = 4096;

fn main() {
    if let Err(error) = execute() {
        eprintln!("pilot_failure={error}");
        std::process::exit(2);
    }
}

fn execute() -> Result<(), String> {
    let mode = std::env::args()
        .nth(1)
        .unwrap_or_else(|| "--dry-run".into());
    let endpoint = if mode == "--preflight" { 128 } else { 4096 };
    let accepted_elapsed: &[u128] = if endpoint == 128 {
        &[0, 64, 128]
    } else {
        &[0, 2048, 4096]
    };
    let probe_elapsed: &[u128] = if endpoint == 128 {
        &[0, 63, 64, 127, 128]
    } else {
        &[0, 2047, 2048, 4095, 4096]
    };
    let residual_elapsed: &[u128] = if endpoint == 128 {
        &[63, 127]
    } else {
        &[2047, 4095]
    };
    let accepted = clocks(accepted_elapsed);
    let probes = clocks(probe_elapsed);
    let residual = clocks(residual_elapsed);
    let family = family_plan(endpoint, &accepted);
    let probe = ProbePlan::new(
        family,
        TestedTimes::new(&probes, probes.len()).map_err(debug)?,
        probes.len(),
        CAP,
    )
    .map_err(debug)?;
    let diagnostic = DiagnosticPlan::new(family, probe, &residual, policy(), CAP).map_err(debug)?;
    let high_plan = high_force_plan(endpoint, accepted[0]);
    let archive_buffer_bytes = (0..6)
        .map(|index| {
            family
                .branch_plan(index)
                .ok_or_else(|| format!("missing branch plan {index}"))
                .and_then(|plan| archive::maximum_encoded_len(plan).map_err(debug))
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .max()
        .ok_or_else(|| "empty branch plan".to_string())?;
    let comparison_hash_scratch_bytes = std::mem::size_of::<ComparisonPlan>()
        + std::mem::size_of::<BandComparison>()
        + 2 * std::mem::size_of::<Sha256>()
        + 2 * std::mem::size_of::<[u8; 32]>();
    let harness_scratch_bytes = comparison_hash_scratch_bytes
        .checked_add(archive_buffer_bytes)
        .and_then(|n| n.checked_add(IO_SCRATCH))
        .ok_or_else(|| "harness resource overflow".to_string())?;
    let combined = diagnostic
        .bounds()
        .joint_storage_bytes
        .checked_add(high_plan.resources().total())
        .and_then(|n| n.checked_add(harness_scratch_bytes))
        .ok_or_else(|| "combined resource overflow".to_string())?;
    println!("source={} case_sha256={} mode={} endpoint={} cap={} combined_bytes={} harness_scratch_bytes={} comparison_hash_scratch_bytes={} archive_buffer_bytes={} io_scratch_bytes={}", env!("PILOT_SOURCE"), CASE_SHA256, mode, endpoint, CAP, combined, harness_scratch_bytes, comparison_hash_scratch_bytes, archive_buffer_bytes, IO_SCRATCH);
    println!(
        "spatial_settings={:?} diagnostic_bounds={:?}",
        family.settings(),
        diagnostic.bounds()
    );
    println!(
        "force_control_base=N12/M24 branch0 high={:?} high_resources={:?}",
        high_plan.settings(),
        high_plan.resources()
    );
    if combined > CAP {
        return Err(format!("combined reservation {combined} exceeds cap {CAP}"));
    }
    if mode == "--dry-run" {
        println!("terminal=dry-run-complete");
        return Ok(());
    }
    if mode != "--preflight" && mode != "--run" {
        return Err(format!("unsupported mode {mode}"));
    }
    fs::create_dir_all("checkpoints").map_err(debug)?;
    run(diagnostic, high_plan, vec![0_u8; archive_buffer_bytes])
}

fn run(
    plan: DiagnosticPlan<'_>,
    high_plan: RunPlan,
    mut archive_buffer: Vec<u8>,
) -> Result<(), String> {
    let started = Instant::now();
    let mut driver = DiagnosticDriver::new(plan).map_err(debug)?;
    let mut high = Run::from_rest(high_plan).map_err(debug)?;
    let (mut events, mut controls) = (0, 0);
    loop {
        let attempted = driver.next_time();
        let before = driver.consumer_work();
        match driver.advance() {
            Ok(Some(event)) => {
                events += 1;
                let accepted = event.accepted().sample().is_some();
                if accepted {
                    archive_branches(
                        &driver,
                        event.clock(),
                        &format!("accepted-event-{events}"),
                        &mut archive_buffer,
                    )?;
                }
                let force = if accepted {
                    advance_high(&mut high, event.clock())?;
                    let base = driver
                        .ordinary()
                        .branch(0)
                        .ok_or_else(|| "missing ordinary branch 0".to_string())?
                        .state();
                    if base.clock() != high.state().clock() {
                        return Err("force-control clock mismatch".to_string());
                    }
                    let comparison = compare(base, high.state()).map_err(debug)?;
                    controls += 1;
                    println!("force_control=N12_M24_to_M48 full={:?} common={:?} newly_resolved={:?} mean_error={:?} base_hash={:02x?} high_hash={:02x?} base_clock={} high_clock={} base_settings={:?} high_settings={:?}", comparison.full, comparison.common, comparison.newly_resolved, comparison.mean_error, hash(base), hash(high.state()), base.clock().elapsed(), high.state().clock().elapsed(), driver.ordinary().branch(0).unwrap().plan().settings(), high.plan().settings());
                    true
                } else {
                    false
                };
                let mut output = std::io::stdout().lock();
                writeln!(output, "event={} wall_seconds={:.6} elapsed={} force_control_emitted={} spatial_raw={:?}", events, started.elapsed().as_secs_f64(), event.clock().elapsed(), force, event).map_err(debug)?;
                output.flush().map_err(debug)?;
            }
            Ok(None) => {
                if events != 5 || controls != 3 {
                    return Err(format!(
                        "incomplete terminal events={events} force_controls={controls}"
                    ));
                }
                println!("terminal=complete events={} force_controls={} wall_seconds={:.6} charged={:?} consumers={:?}", events, controls, started.elapsed().as_secs_f64(), driver.charged_work(), driver.consumer_work());
                return Ok(());
            }
            Err(error) => {
                let after = driver.consumer_work();
                let clock =
                    attempted.ok_or_else(|| "failure without attempted clock".to_string())?;
                let phase = failure_phase(before, after);
                let archived = archive_branches(
                    &driver,
                    clock,
                    &format!("failed-{phase}-after-return"),
                    &mut archive_buffer,
                )?;
                println!("terminal=diagnostic-error events={} force_controls={} wall_seconds={:.6} attempted_clock={} inferred_phase={} archived_branches={} error={:?} charged={:?} consumers_before={:?} consumers_after={:?}", events, controls, started.elapsed().as_secs_f64(), clock.elapsed(), phase, archived, error, driver.charged_work(), before, after);
                return Err(format!(
                    "diagnostic error at {} in {phase}: {error:?}",
                    clock.elapsed()
                ));
            }
        }
    }
}

fn advance_high(run: &mut Run, clock: TickClock) -> Result<(), String> {
    while run.state().clock().elapsed() < clock.elapsed() {
        let outcome = run.step().map_err(debug)?;
        if !matches!(outcome, Outcome::Committed(_)) {
            return Err(format!("high-force stopped: {outcome:?}"));
        }
    }
    if run.state().clock() != clock {
        return Err("high-force overshot accepted clock".to_string());
    }
    Ok(())
}

fn archive_branches(
    driver: &DiagnosticDriver<'_>,
    clock: TickClock,
    tag: &str,
    output: &mut [u8],
) -> Result<usize, String> {
    let mut written = 0;
    for index in 0..6 {
        let run = driver
            .ordinary()
            .branch(index)
            .ok_or_else(|| format!("missing ordinary branch {index}"))?;
        if run.state().clock() != clock {
            continue;
        }
        let bytes = archive::write(run, output).map_err(debug)?;
        let digest: [u8; 32] = Sha256::digest(&output[..bytes]).into();
        let path = format!(
            "checkpoints/{tag}-clock{}-branch{index}.bin",
            clock.elapsed()
        );
        fs::write(Path::new(&path), &output[..bytes]).map_err(debug)?;
        println!(
            "checkpoint path={} bytes={} sha256={:02x?} state_clock={} settings={:?} origin={:?}",
            path,
            bytes,
            digest,
            run.state().clock().elapsed(),
            run.plan().settings(),
            run.origin()
        );
        written += 1;
    }
    Ok(written)
}

fn failure_phase(before: DiagnosticConsumerWork, after: DiagnosticConsumerWork) -> &'static str {
    if after.binding.attempts > before.binding.attempts {
        "binding"
    } else if after.regional.attempts > before.regional.attempts {
        "regional"
    } else if after.reference.attempts > before.reference.attempts {
        "reference"
    } else if after.pressure.attempts > before.pressure.attempts {
        "pressure"
    } else if after.physical.attempts > before.physical.attempts {
        "physical"
    } else if after.residual.attempts > before.residual.attempts {
        "residual"
    } else if after.probes.attempts > before.probes.attempts {
        "probes"
    } else {
        "ordinary"
    }
}

fn compare(
    a: &SpectralState,
    b: &SpectralState,
) -> Result<BandComparison, nsbu_solver::SolverError> {
    ComparisonPlan::new(a.plan().domain(), b.plan().domain())?.compare(
        [a.component(0)?, a.component(1)?, a.component(2)?],
        [b.component(0)?, b.component(1)?, b.component(2)?],
    )
}

fn hash(state: &SpectralState) -> [u8; 32] {
    let mut hasher = Sha256::new();
    for axis in 0..3 {
        for value in state.component(axis).unwrap() {
            hasher.update(value.re.to_bits().to_le_bytes());
            hasher.update(value.im.to_bits().to_le_bytes());
        }
    }
    hasher.finalize().into()
}

fn family_plan<'a>(endpoint: u128, times: &'a [TickClock]) -> FamilyPlan<'a> {
    FamilyPlan::new(
        FamilySettings {
            grids: [12, 16, 24],
            steps: [64, 32, 16],
            force: ForceSettings {
                samples: Layout::new([24; 3]).unwrap(),
                workers: 12,
            },
            endpoint,
            tolerances: tolerances(),
            advective_limit: 0.3,
        },
        TestedTimes::new(times, times.len()).unwrap(),
        CAP,
    )
    .unwrap()
}

fn high_force_plan(endpoint: u128, initial: TickClock) -> RunPlan {
    RunPlan::from_rest(
        RunSettings {
            domain: Domain::new([12; 3], [1.0; 3], 1.0).unwrap(),
            force: ForceSettings {
                samples: Layout::new([48; 3]).unwrap(),
                workers: 12,
            },
            initial_clock: initial,
            configuration: Configuration {
                method: Method::CoxMatthews,
                limits: RunLimits {
                    endpoint,
                    step_ticks: 16,
                    maximum_attempts: usize::try_from(endpoint / 16).unwrap(),
                },
                tolerances: tolerances(),
            },
            advective_limit: 0.3,
        },
        CAP,
    )
    .unwrap()
}

fn policy() -> DiagnosticSettings {
    DiagnosticSettings {
        physical_samples: Layout::new([24; 3]).unwrap(),
        pressure_samples: Layout::new([48; 3]).unwrap(),
        reference_samples: Layout::new([24; 3]).unwrap(),
        physical_floors: [1e-8, 1e-7, 1e-6, 1e-7],
        pressure_floors: [1e-8, 1e-7],
        reference_floors: [1e-8, 1e-7, 1e-6, 1e-7],
        regional_root_budget: 128,
    }
}

fn tolerances() -> Tolerances {
    Tolerances {
        absolute: [1e-5, 1e-4],
        relative: [0.0; 2],
    }
}
fn clocks(values: &[u128]) -> Vec<TickClock> {
    values
        .iter()
        .map(|&elapsed| TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap())
        .collect()
}
fn debug(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}
