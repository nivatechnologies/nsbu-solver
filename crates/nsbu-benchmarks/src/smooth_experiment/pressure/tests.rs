//! Independent Fourier/Taylor–Green controls exercise padding, physical pressure gauge and force band.
use super::*;
use crate::smooth_experiment::{FamilyPlan, FamilySettings};
use nsbu_solver::{integrators::indicator::Tolerances, verification::times::TestedTimes};

fn plan<'a>(clocks: &'a [TickClock]) -> PressureFamilyPlan<'a> {
    let family = FamilyPlan::new(
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
        },
        TestedTimes::new(clocks, 3).unwrap(),
        128 * 1024 * 1024,
    )
    .unwrap();
    PressureFamilyPlan::new(
        family,
        Layout::new([24; 3]).unwrap(),
        [1.0; 2],
        3,
        128 * 1024 * 1024,
    )
    .unwrap()
}
fn pair(layout: Layout, values: &mut [Complex64], mode: [isize; 3], value: Complex64) {
    for (wave, z) in [(mode, value), (mode.map(|v| -v), value.conj())] {
        values[layout.locate(wave).unwrap().0] = z;
    }
}

#[test]
fn padded_taylor_green_retains_pressure_outside_the_original_velocity_band() {
    let clocks = [0, 64, 128].map(|t| TickClock::restore(-16, 512, t, 512 - t).unwrap());
    let mut work = PressureFamilyWorkspace::new(plan(&clocks)).unwrap();
    let coarse = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let mut velocity = field(coarse.layout().half_len()).unwrap();
    for sign in [-1, 1] {
        pair(
            coarse.layout(),
            &mut velocity[0],
            [1, sign, 0],
            Complex64::new(0.0, -0.25),
        );
        pair(
            coarse.layout(),
            &mut velocity[1],
            [1, sign, 0],
            Complex64::new(0.0, 0.25 * sign as f64),
        );
    }
    work.construct_values(coarse, velocity.each_ref().map(Vec::as_slice), 0)
        .unwrap();
    let output = ConservativeWorkspace::diagnostic_domain(work.plan.source)
        .unwrap()
        .layout();
    let mut expected = filled(output.half_len()).unwrap();
    for wave in [[2, 0, 0], [0, 2, 0]] {
        pair(output, &mut expected, wave, Complex64::new(0.125, 0.0));
    }
    for (actual, expected) in work.pressure[0].iter().zip(expected) {
        let delta = *actual - expected;
        assert!(delta.re.hypot(delta.im) < 2e-15);
    }
    assert_eq!(work.pressure[0][0], Complex64::new(0.0, 0.0));
    let scalar = work.compare(PhysicalQuantity::Scalar, 0).unwrap();
    let gradient = work.compare(PhysicalQuantity::ScalarGradient, 1).unwrap();
    assert!((scalar.rms_error - 0.25).abs() < 1e-14);
    assert!((gradient.rms_error - std::f64::consts::PI).abs() < 1e-13);
}

#[test]
fn force_only_pressure_retains_modes_beyond_the_finest_velocity_band() {
    let clocks = [0, 64, 128].map(|t| TickClock::restore(-16, 512, t, 512 - t).unwrap());
    let mut work = PressureFamilyWorkspace::new(plan(&clocks)).unwrap();
    let output = ConservativeWorkspace::diagnostic_domain(work.plan.source)
        .unwrap()
        .layout();
    pair(
        output,
        &mut work.force[0],
        [10, 0, 0],
        Complex64::new(10.0 * std::f64::consts::PI, 0.0),
    );
    let velocity = field(work.plan.source.layout().half_len()).unwrap();
    work.construct_values(work.plan.source, velocity.each_ref().map(Vec::as_slice), 0)
        .unwrap();
    let mut expected = filled(output.half_len()).unwrap();
    pair(output, &mut expected, [10, 0, 0], Complex64::new(0.0, -0.5));
    for (actual, expected) in work.pressure[0].iter().zip(expected) {
        let delta = *actual - expected;
        assert!(delta.re.hypot(delta.im) < 2e-15);
    }
    assert_eq!(work.pressure[0][0], Complex64::new(0.0, 0.0));
}
