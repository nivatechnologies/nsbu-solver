//! Independent full-force Poisson control; common force cancels in pair-only checks.
use super::*;
use crate::{
    runtime_force::ForceSettings,
    v2_experiment::{FamilyPlan, FamilySettings},
};
use nsbu_solver::{
    domain::{Layout, TickClock},
    integrators::{forcing::PrescribedForce, indicator::Tolerances},
    verification::times::TestedTimes,
};

#[test]
fn zero_velocity_uses_independent_full_band_force_and_mean_zero_pressure() {
    let clocks =
        [0, 64, 128].map(|elapsed| TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap());
    let family = FamilyPlan::new(
        FamilySettings {
            grids: [4, 8, 12],
            steps: [64, 32, 16],
            force: ForceSettings {
                samples: Layout::new([12; 3]).unwrap(),
                workers: 0,
            },
            endpoint: 128,
            tolerances: Tolerances {
                absolute: [1e-5, 1e-4],
                relative: [0.0; 2],
            },
            advective_limit: 0.3,
        },
        TestedTimes::new(&clocks, 3).unwrap(),
        256 * 1024 * 1024,
    )
    .unwrap();
    let source = family.branches[2].resources().domain();
    let diagnostic = ConservativeWorkspace::diagnostic_domain(source).unwrap();
    let plan = PressureFamilyPlan::new(
        family,
        diagnostic.layout(),
        [1e-8, 1e-7],
        3,
        256 * 1024 * 1024,
    )
    .unwrap();
    let mut workspace = PressureFamilyWorkspace::new(plan).unwrap();
    let clock = clocks[2];
    workspace
        .provider
        .evaluate(
            clock,
            workspace.plan.provider_limits,
            workspace.force.each_mut().map(Vec::as_mut_slice),
        )
        .unwrap();
    let zero = [
        vec![Complex64::new(0.0, 0.0); source.layout().half_len()],
        vec![Complex64::new(0.0, 0.0); source.layout().half_len()],
        vec![Complex64::new(0.0, 0.0); source.layout().half_len()],
    ];
    workspace
        .construct_values(source, zero.each_ref().map(Vec::as_slice), 0)
        .unwrap();
    assert_eq!(workspace.pressure[0][0], Complex64::new(0.0, 0.0));
    assert!(workspace.pressure[0]
        .iter()
        .any(|value| value.l1_norm() > 0.0));
    let layout = diagnostic.layout();
    let limits = V2Force::preflight(diagnostic, layout).unwrap();
    let mut independent = V2Force::new(diagnostic, layout, limits.storage_bytes).unwrap();
    let mut force = field(layout.half_len()).unwrap();
    independent
        .evaluate(clock, limits, force.each_mut().map(Vec::as_mut_slice))
        .unwrap();
    assert_eq!(workspace.force, force);
    compare_poisson(layout, &force, &workspace.pressure[0]);
}

fn compare_poisson(layout: Layout, force: &Field, pressure: &[Complex64]) {
    let [nx, ny, nz] = layout.dimensions();
    let mut nonzero_beyond_velocity = 0;
    for x in 0..nx {
        for y in 0..ny {
            for z in 0..=nz / 2 {
                let index = (x * ny + y) * (nz / 2 + 1) + z;
                let mode = [signed(x, nx), signed(y, ny), z as isize];
                if x == nx / 2 || y == ny / 2 || z == nz / 2 || mode == [0; 3] {
                    assert_eq!(pressure[index], Complex64::new(0.0, 0.0));
                    continue;
                }
                let k = mode.map(|m| std::f64::consts::TAU * m as f64);
                let squared = k.iter().map(|v| v * v).sum::<f64>();
                let mut divergence = Complex64::new(0.0, 0.0);
                let mut term_scale = 0.0;
                for axis in 0..3 {
                    let term = force[axis][index] * k[axis];
                    divergence += term;
                    term_scale += term.l1_norm();
                }
                let expected = -Complex64::i() * divergence / squared;
                let allowance = 2e-13 * term_scale / squared + 1e-30;
                assert!(
                    (pressure[index] - expected).l1_norm() < allowance,
                    "mode={mode:?} measured={:?} expected={expected:?} allowance={allowance:e}",
                    pressure[index]
                );
                if mode.iter().any(|m| m.unsigned_abs() >= nx / 4)
                    && expected.l1_norm() > 100.0 * allowance
                {
                    nonzero_beyond_velocity += 1;
                }
            }
        }
    }
    assert!(
        nonzero_beyond_velocity > 0,
        "full force pressure must survive beyond the velocity band"
    );
}

fn signed(index: usize, n: usize) -> isize {
    if index < n / 2 {
        index as isize
    } else {
        index as isize - n as isize
    }
}
