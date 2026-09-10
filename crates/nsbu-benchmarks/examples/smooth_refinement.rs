//! Bounded six-trajectory comparison walkthrough; its outputs are diagnostic samples.
use nsbu_benchmarks::smooth_experiment::{
    physical::{PhysicalFamilyPlan, PhysicalFamilyWorkspace, QuantityRefinement},
    pressure::{PressureFamilyPlan, PressureFamilyWorkspace},
    reconstruction::ReconstructionWorkspace,
    residual::{ResidualBounds, ResidualWorkspace},
    FamilyError, FamilyPlan, FamilySettings, SmoothFamily,
};
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
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
    evolve(admit(&clocks)?)
}

struct Plan<'a> {
    family: FamilyPlan<'a>,
    physical: PhysicalFamilyPlan<'a>,
    pressure: PressureFamilyPlan<'a>,
    reconstruction_bytes: usize,
    residual: ResidualBounds,
    domain: Domain,
}

// Aggregate every simultaneous owner before allocating the first one.
fn admit(clocks: &[TickClock]) -> Result<Plan<'_>, FamilyError> {
    let plan = FamilyPlan::new(settings(), TestedTimes::new(clocks, 3)?, CAP)?;
    let domain = Domain::new([12; 3], [1.0; 3], 1.0)?;
    let physical_plan = PhysicalFamilyPlan::new(
        plan,
        Layout::new([24; 3])?,
        [1e-8, 1e-7, 1e-6, 1e-7],
        3,
        CAP,
    )?;
    let pressure_plan = PressureFamilyPlan::new(plan, Layout::new([24; 3])?, [1e-8, 1e-7], 3, CAP)?;
    let base = physical_plan
        .bounds()
        .joint_storage_bytes
        .checked_add(pressure_plan.bounds().storage_bytes)
        .ok_or(SolverError::SizeOverflow)?;
    let (bytes, reconstruction_bytes, residual_bounds) = diagnostic_budget(base, domain, CAP)?;
    println!("CyclicSine diagnostic experiment; independently evolved from rest; accepted concentrating windows=0");
    println!(
        "grids=[4,8,12] steps_ticks=[64,32,16] quantum=2^-16 endpoint_ticks=128 methods=CM,HO"
    );
    println!(
        "preflight storage_bytes={bytes} cap={CAP} family_work={:?} residual_work={:?}",
        plan.bounds(),
        residual_bounds.work
    );
    Ok(Plan {
        family: plan,
        physical: physical_plan,
        pressure: pressure_plan,
        reconstruction_bytes,
        residual: residual_bounds,
        domain,
    })
}

fn evolve(plan: Plan<'_>) -> Result<(), FamilyError> {
    let mut family = SmoothFamily::new(plan.family)?;
    let mut physical = PhysicalFamilyWorkspace::new(plan.physical)?;
    let mut pressure = PressureFamilyWorkspace::new(plan.pressure)?;
    let mut reconstruction =
        ReconstructionWorkspace::new(plan.family, 1, plan.reconstruction_bytes)?;
    let mut residual = ResidualWorkspace::new(plan.domain, 1, plan.residual.storage_bytes)?;
    while let Some(sample) = family.advance()? {
        println!(
            "tick={} space_H1={:?} time_H1={:?} method_H1={:.12e}",
            sample.clock().elapsed(),
            sample.space().map(|n| n.full.h1),
            sample.time().map(|n| n.full.h1),
            sample.method().full.h1
        );
        let measured = physical.measure(&family)?;
        print_quantities(measured.clock(), measured.quantities());
        let measured = pressure.measure(&family)?;
        print_quantities(measured.clock(), measured.quantities());
    }
    println!(
        "physical_work={:?} pressure_work={:?}",
        physical.charged_work(),
        pressure.charged_work()
    );
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

fn print_quantities(clock: TickClock, quantities: &[QuantityRefinement]) {
    for item in quantities {
        println!(
            "tick={} {:?} spatial_RMS={:?} temporal_RMS={:?} method_RMS={:.12e}",
            clock.elapsed(),
            item.quantity,
            item.space.map(|e| e.rms_error),
            item.time.map(|e| e.rms_error),
            item.method.rms_error
        );
    }
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
