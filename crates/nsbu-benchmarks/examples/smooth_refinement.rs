//! Bounded six-trajectory comparison walkthrough; its outputs are diagnostic samples.
use nsbu_benchmarks::smooth_experiment::{
    reconstruction::ReconstructionWorkspace,
    residual::{ResidualBounds, ResidualWorkspace},
    FamilyError, FamilyPlan, FamilySettings, SmoothFamily,
};
use nsbu_solver::{
    domain::{Domain, TickClock},
    integrators::indicator::Tolerances,
    verification::times::TestedTimes,
    SolverError,
};
const CAP: usize = 128 * 1024 * 1024;
fn clock(t: u128) -> Result<TickClock, SolverError> {
    TickClock::restore(-16, 512, t, 512 - t)
}
fn settings() -> FamilySettings {
    FamilySettings {
        grids: [4, 8, 12],
        steps: [64, 32, 16],
        viscosity: 1.0,
        endpoint: 128,
        tolerances: Tolerances {
            absolute: [1e-2; 2],
            relative: [0.0; 2],
        },
        advective_limit: 0.3,
    }
}
fn main() -> Result<(), FamilyError> {
    let clocks = [clock(0)?, clock(64)?, clock(128)?];
    let plan = FamilyPlan::new(settings(), TestedTimes::new(&clocks, 3)?, CAP)?;
    let domain = Domain::new([12; 3], [1.0; 3], 1.0)?;
    let (bytes, reconstruction_bytes, residual_bounds) =
        diagnostic_budget(plan.bounds().storage_bytes, domain, CAP)?;
    println!("CyclicSine diagnostic experiment; independently evolved from rest; accepted concentrating windows=0");
    println!(
        "grids=[4,8,12] steps_ticks=[64,32,16] quantum=2^-16 endpoint_ticks=128 methods=CM,HO"
    );
    println!(
        "preflight storage_bytes={bytes} cap={CAP} family_work={:?} residual_work={:?}",
        plan.bounds(),
        residual_bounds.work
    );
    let mut family = SmoothFamily::new(plan)?;
    let mut reconstruction = ReconstructionWorkspace::new(plan, 1, reconstruction_bytes)?;
    let mut residual = ResidualWorkspace::new(domain, 1, residual_bounds.storage_bytes)?;
    while let Some(sample) = family.advance()? {
        println!(
            "tick={} space_H1={:?} time_H1={:?} method_H1={:.12e}",
            sample.clock().elapsed(),
            sample.space().map(|n| n.full.h1),
            sample.time().map(|n| n.full.h1),
            sample.method().full.h1
        );
    }
    let probe = clock(127)?;
    let comparison = reconstruction.measure(&family, probe)?;
    let measured = residual.measure(family.branch(2).ok_or(FamilyError::InvalidFamily)?, probe)?;
    println!(
        "offstage_tick=127 reconstruction_H1={:?} derivative_H1={:?}",
        comparison.values().map(|n| n.full.h1),
        comparison.derivatives().map(|n| n.full.h1)
    );
    println!(
        "full_double_band_residual={:?} origin={:?}",
        measured.norms(),
        measured.origin()
    );
    println!("Incomplete qualification: force/reference/arithmetic/sampling/quadrature and full observable studies remain.");
    Ok(())
}
#[test]
fn documented_comparison_walkthrough_runs_within_its_declared_cap() {
    main().expect("the documented bounded family must finish its diagnostic samples");
}

// Joint admission includes diagnostic workspaces in addition to all six complete owners.
fn diagnostic_budget(
    family_bytes: usize,
    domain: Domain,
    cap: usize,
) -> Result<(usize, usize, ResidualBounds), SolverError> {
    let reconstruction_bytes = ReconstructionWorkspace::reservation(domain, 1)?.0;
    let residual_bounds = ResidualWorkspace::reservation(domain, 1)?;
    let bytes = family_bytes
        .checked_add(reconstruction_bytes)
        .and_then(|n| n.checked_add(residual_bounds.storage_bytes))
        .ok_or(SolverError::SizeOverflow)?;
    if bytes > cap {
        return Err(SolverError::ResourceLimit);
    }
    Ok((bytes, reconstruction_bytes, residual_bounds))
}
#[test]
fn joint_cap_refuses_unreserved_diagnostic_buffers_and_overflow() {
    let domain = Domain::new([12; 3], [1.0; 3], 1.0).unwrap();
    assert!(diagnostic_budget(CAP, domain, CAP).is_err());
    assert!(diagnostic_budget(usize::MAX, domain, usize::MAX).is_err());
}
