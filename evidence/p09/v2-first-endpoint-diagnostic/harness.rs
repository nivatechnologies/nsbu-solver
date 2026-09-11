//! Allocation-free admission report for a proposed first-endpoint v2 coordinator pilot.
use nsbu_benchmarks::{
    runtime_force::ForceSettings,
    v2_experiment::{
        diagnostic::{DiagnosticPlan, DiagnosticSettings},
        probes::ProbePlan,
        FamilyPlan, FamilySettings,
    },
    CASE_SHA256,
};
use nsbu_solver::{
    domain::{Layout, TickClock},
    integrators::indicator::Tolerances,
    verification::times::TestedTimes,
};
use std::{io::Write, time::Instant};

const CAP: usize = 1024 * 1024 * 1024;

fn main() {
    let run_startup = std::env::args().any(|argument| argument == "--run-startup");
    let run_full = std::env::args().any(|argument| argument == "--run-full");
    for profile in [
        Profile {
            endpoint: 128,
            accepted: &[0, 64, 128],
            probes: &[0, 63, 64, 127, 128],
            residual: &[63, 127],
        },
        Profile {
            endpoint: 4096,
            accepted: &[0, 2048, 4096],
            probes: &[0, 2047, 2048, 4095, 4096],
            residual: &[2047, 4095],
        },
    ] {
        let accepted = clocks(profile.accepted);
        let manifest = clocks(profile.probes);
        let residual = clocks(profile.residual);
        let family = FamilyPlan::new(
            settings(profile.endpoint),
            TestedTimes::new(&accepted, accepted.len()).unwrap(),
            CAP,
        )
        .unwrap();
        let probes = ProbePlan::new(
            family,
            TestedTimes::new(&manifest, manifest.len()).unwrap(),
            manifest.len(),
            CAP,
        )
        .unwrap();
        let plan = DiagnosticPlan::new(family, probes, &residual, policy(), CAP).unwrap();
        println!("case=similarity-mms-v2 sha256={CASE_SHA256} endpoint={} cap={} joint_bytes={} coordinator={:?}", profile.endpoint, CAP, plan.bounds().joint_storage_bytes, plan.bounds().work);
        println!("family={:?}\nprobes={:?}\nphysical={:?}\npressure={:?}\nreference={:?}\nregional={:?}\nresidual={:?}\nbinding={:?}", family.bounds(), plan.bounds().probes, plan.bounds().physical, plan.bounds().pressure, plan.bounds().reference, plan.bounds().regional, plan.bounds().residual, plan.bounds().binding);
        if (run_startup && profile.endpoint == 128) || (run_full && profile.endpoint == 4096) {
            run_driver(plan, profile.endpoint);
        }
    }
}

fn run_driver(plan: DiagnosticPlan, endpoint: u128) {
    let started = Instant::now();
    let mut driver = nsbu_benchmarks::v2_experiment::diagnostic::DiagnosticDriver::new(plan).unwrap();
    let mut events = 0;
    loop {
        match driver.advance() {
            Ok(Some(event)) => {
                events += 1;
                let mut output = std::io::stdout().lock();
                writeln!(
                    output,
                    "event={events} wall_seconds={:.3} elapsed={} raw={event:?}",
                    started.elapsed().as_secs_f64(),
                    event.clock().elapsed(),
                )
                .unwrap();
                output.flush().unwrap();
            }
            Ok(None) => {
                println!(
                    "terminal=complete endpoint={endpoint} events={events} wall_seconds={:.3} charged={:?} consumers={:?}",
                    started.elapsed().as_secs_f64(),
                    driver.charged_work(),
                    driver.consumer_work()
                );
                break;
            }
            Err(error) => {
                println!(
                    "terminal=error endpoint={endpoint} events={events} wall_seconds={:.3} error={error:?} charged={:?} consumers={:?}",
                    started.elapsed().as_secs_f64(),
                    driver.charged_work(),
                    driver.consumer_work()
                );
                break;
            }
        }
    }
}
struct Profile {
    endpoint: u128,
    accepted: &'static [u128],
    probes: &'static [u128],
    residual: &'static [u128],
}
fn clocks(values: &[u128]) -> Vec<TickClock> {
    values
        .iter()
        .map(|&elapsed| TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap())
        .collect()
}
fn settings(endpoint: u128) -> FamilySettings {
    FamilySettings {
        grids: [8, 12, 16],
        steps: [64, 32, 16],
        force: ForceSettings {
            samples: Layout::new([16; 3]).unwrap(),
            workers: 8,
        },
        endpoint,
        tolerances: Tolerances {
            absolute: [1e-5, 1e-4],
            relative: [0.0; 2],
        },
        advective_limit: 0.3,
    }
}
fn policy() -> DiagnosticSettings {
    DiagnosticSettings {
        physical_samples: Layout::new([16; 3]).unwrap(),
        pressure_samples: Layout::new([32; 3]).unwrap(),
        reference_samples: Layout::new([16; 3]).unwrap(),
        physical_floors: [1e-8, 1e-7, 1e-6, 1e-7],
        pressure_floors: [1e-8, 1e-7],
        reference_floors: [1e-8, 1e-7, 1e-6, 1e-7],
        regional_root_budget: 128,
    }
}
