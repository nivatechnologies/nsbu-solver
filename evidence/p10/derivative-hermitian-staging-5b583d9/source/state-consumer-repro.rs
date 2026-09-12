use nsbu_benchmarks::{
    runtime_force::ForceSettings,
    v2_run::{archive, Plan, Run, Settings},
};
use nsbu_solver::{
    checkpoint::CheckpointError,
    diagnostics::{
        derivatives::{Derivative, DerivativeWorkspace},
        physical::{PhysicalComparisonWorkspace, PhysicalField, PhysicalQuantity},
    },
    domain::{Domain, Layout, TickClock},
    experiment::control::{Configuration, Outcome},
    integrators::{indicator::Tolerances, method::Method, trajectory::RunLimits},
    SolverError,
};
use sha2::{Digest, Sha256};
use std::{fs, io::Write};

const CAP: usize = 256 * 1024 * 1024;
const STOP: u128 = 2048;

#[derive(Debug)]
enum Error {
    Solver(SolverError),
    Checkpoint(CheckpointError),
    Io(std::io::Error),
    Message(String),
}
impl From<SolverError> for Error {
    fn from(value: SolverError) -> Self {
        Self::Solver(value)
    }
}
impl From<CheckpointError> for Error {
    fn from(value: CheckpointError) -> Self {
        Self::Checkpoint(value)
    }
}
impl From<std::io::Error> for Error {
    fn from(value: std::io::Error) -> Self {
        Self::Io(value)
    }
}
impl std::fmt::Display for Error {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{self:?}")
    }
}
impl std::error::Error for Error {}

fn main() -> Result<(), Error> {
    let domain = Domain::new([12; 3], [1.0; 3], 1.0)?;
    let settings = Settings {
        domain,
        force: ForceSettings {
            samples: Layout::new([24; 3])?,
            workers: 12,
        },
        initial_clock: TickClock::from_rest(-20, 8192)?,
        configuration: Configuration {
            method: Method::CoxMatthews,
            limits: RunLimits {
                endpoint: 4096,
                step_ticks: 16,
                maximum_attempts: 256,
            },
            tolerances: Tolerances {
                absolute: [1e-5, 1e-4],
                relative: [0.0; 2],
            },
        },
        advective_limit: 0.3,
    };
    let plan = Plan::from_rest(settings, CAP)?;
    let samples = Layout::new([24; 3])?;
    let physical_bytes = PhysicalComparisonWorkspace::reservation(domain, domain, samples)?;
    let archive_bytes = archive::maximum_encoded_len(plan)?;
    let combined = plan
        .resources()
        .total()
        .checked_add(physical_bytes)
        .and_then(|n| n.checked_add(archive_bytes))
        .and_then(|n| n.checked_add(16 * 64))
        .ok_or_else(|| Error::Message("size overflow".into()))?;
    if combined > CAP {
        return Err(Error::Message("combined reservation exceeds cap".into()));
    }
    println!("source={} settings={settings:?} run_resources={:?} physical_bytes={physical_bytes} maximum_archive_bytes={archive_bytes} allocator_allowance={} combined_bytes={combined} cap={CAP}", env!("REPRO_SOURCE"), plan.resources(), 16*64);
    let mut run = Run::from_rest(plan)?;
    while run.state().clock().elapsed() < STOP {
        match run.step()? {
            Outcome::Committed(_) => {}
            outcome => {
                return Err(Error::Message(format!(
                    "trajectory stopped at {:?}: {outcome:?}",
                    run.state().clock()
                )))
            }
        }
    }
    if run.state().clock().elapsed() != STOP {
        return Err(Error::Message("wrong stop clock".into()));
    }
    let mut bytes = vec![0u8; archive_bytes];
    let written = archive::write(&run, &mut bytes)?;
    bytes.truncate(written);
    fs::write("checkpoint-2048.bin", &bytes)?;
    let hash = Sha256::digest(&bytes);
    println!(
        "checkpoint_saved=true bytes={written} sha256={hash:x} clock={:?} origin={:?}",
        run.state().clock(),
        run.origin()
    );
    std::io::stdout().flush()?;

    let field = PhysicalField::Vector([
        run.state().component(0)?,
        run.state().component(1)?,
        run.state().component(2)?,
    ]);
    let mut workspace = PhysicalComparisonWorkspace::new(domain, domain, samples, physical_bytes)?;
    for (quantity, floor) in [
        (PhysicalQuantity::Vector, 1e-8),
        (PhysicalQuantity::Gradient, 1e-7),
        (PhysicalQuantity::Hessian, 1e-6),
        (PhysicalQuantity::Vorticity, 1e-7),
    ] {
        let result = workspace.compare(field, field, quantity, floor)?;
        println!(
            "physical quantity={quantity:?} global={:?} transforms={}",
            result.global(),
            result.scalar_transforms()
        );
    }
    for axis in 0..3 {
        let mut derivative = DerivativeWorkspace::new(domain, samples, physical_bytes)?;
        for orders in [
            [0, 0, 0],
            [1, 0, 0],
            [0, 1, 0],
            [0, 0, 1],
            [2, 0, 0],
            [1, 1, 0],
            [1, 0, 1],
            [0, 2, 0],
            [0, 1, 1],
            [0, 0, 2],
        ] {
            let result =
                derivative.sample(run.state().component(axis)?, Derivative::new(orders)?)?;
            let maximum = result.values.iter().fold(0.0_f64, |a, &b| a.max(b.abs()));
            println!(
                "derivative axis={axis} orders={orders:?} samples={} max_abs={maximum:.17e}",
                result.values.len()
            );
        }
    }
    println!("terminal=complete clock={STOP}");
    Ok(())
}
