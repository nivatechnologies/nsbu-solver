//! Sampled maxima can plateau below the actual peak; spatial locations stay unaligned.
mod diagnostics_support;
use diagnostics_support::{close, mode, slices, zero};
use nsbu_solver::{
    diagnostics::sampling::SamplingWorkspace,
    domain::{Domain, Layout},
    Complex64, SolverError,
};

fn shifted_sine(domain: Domain, axis: usize) -> [Vec<Complex64>; 3] {
    let mut field = zero(domain.layout());
    let phase = std::f64::consts::PI / 8.0;
    let mut wave = [0; 3];
    wave[axis] = 1;
    let mut value = [Complex64::new(0.0, 0.0); 3];
    value[(axis + 1) % 3] = Complex64::new(phase.sin() / 2.0, -phase.cos() / 2.0);
    mode(domain.layout(), &mut field, wave, value);
    field
}

#[test]
fn two_equal_sampled_maxima_do_not_establish_a_spatial_supremum() {
    let domain = Domain::new([4; 3], [1.0, 2.0, 3.0], 1.0).unwrap();
    let field = shifted_sine(domain, 0);
    for n in [4, 8, 16] {
        let layout = Layout::new([n, 6, 8]).unwrap();
        let cap = SamplingWorkspace::reservation(domain, layout).unwrap();
        let mut work = SamplingWorkspace::new(domain, layout, cap).unwrap();
        let report = work.sample(slices(&field)).unwrap();
        let expected = if n == 16 {
            1.0
        } else {
            (std::f64::consts::PI / 8.0).cos()
        };
        close(report.velocity_maximum.value, expected);
        close(
            report.vorticity_maximum.value,
            std::f64::consts::TAU * expected,
        );
        assert_eq!(report.dimensions, [n, 6, 8]);
        assert_eq!(report.scalar_transforms, 6);
        for index in 0..layout.real_len() {
            let x = (index / 48) as f64 / n as f64;
            let phase = std::f64::consts::TAU * x + std::f64::consts::PI / 8.0;
            close(report.velocity[1][index], phase.sin());
            close(
                report.vorticity[2][index],
                std::f64::consts::TAU * phase.cos(),
            );
            assert_eq!(report.velocity[0][index], 0.0);
            assert_eq!(report.vorticity[0][index], 0.0);
        }
    }
}

#[test]
fn maximum_locations_use_each_physical_axis_without_recentering() {
    let domain = Domain::new([4; 3], [1.0, 2.0, 3.0], 1.0).unwrap();
    let layout = Layout::new([16; 3]).unwrap();
    let mut work = SamplingWorkspace::new(
        domain,
        layout,
        SamplingWorkspace::reservation(domain, layout).unwrap(),
    )
    .unwrap();
    for axis in 0..3 {
        let field = shifted_sine(domain, axis);
        let report = work.sample(slices(&field)).unwrap();
        for maximum in [report.velocity_maximum, report.vorticity_maximum] {
            assert!(maximum.ties > 0);
            for (coordinate, length) in domain.lengths().into_iter().enumerate() {
                close(
                    maximum.position[coordinate],
                    length * maximum.grid_index[coordinate] as f64 / 16.0,
                );
            }
        }
        let phase = std::f64::consts::TAU * report.velocity_maximum.position[axis]
            / domain.lengths()[axis]
            + std::f64::consts::PI / 8.0;
        close(phase.sin().abs(), 1.0);
        close(
            report.vorticity_maximum.value,
            std::f64::consts::TAU / domain.lengths()[axis],
        );
    }
    let field = zero(domain.layout());
    let report = work.sample(slices(&field)).unwrap();
    assert_eq!(report.velocity_maximum.value, 0.0);
    assert_eq!(report.velocity_maximum.ties, layout.real_len());
    assert_eq!(report.velocity_maximum.grid_index, [0; 3]);
    assert_eq!(report.vorticity_maximum.ties, layout.real_len());
}

#[test]
fn incompatible_grids_caps_and_inputs_are_refused() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let small = Layout::new([2, 4, 4]).unwrap();
    assert_eq!(
        SamplingWorkspace::reservation(domain, small).unwrap_err(),
        SolverError::InvalidDomain
    );
    let layout = Layout::new([6; 3]).unwrap();
    let cap = SamplingWorkspace::reservation(domain, layout).unwrap();
    assert_eq!(
        SamplingWorkspace::new(domain, layout, cap - 1).unwrap_err(),
        SolverError::ResourceLimit
    );
    let mut work = SamplingWorkspace::new(domain, layout, cap).unwrap();
    let mut field = zero(domain.layout());
    field[0].pop();
    assert_eq!(
        work.sample(slices(&field)).unwrap_err(),
        SolverError::InvalidPayload
    );
    let mut field = zero(domain.layout());
    field[2][0] = Complex64::new(f64::NAN, 0.0);
    assert_eq!(
        work.sample(slices(&field)).unwrap_err(),
        SolverError::InvalidSpectrum
    );
    let unsupported = Layout::new([10; 3]).unwrap();
    assert_eq!(
        SamplingWorkspace::reservation(domain, unsupported).unwrap_err(),
        SolverError::InvalidDomain
    );
}

#[test]
fn vector_magnitudes_preserve_means_and_refuse_overflow_without_returning_views() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let layout = Layout::new([6; 3]).unwrap();
    let cap = SamplingWorkspace::reservation(domain, layout).unwrap();
    let mut work = SamplingWorkspace::new(domain, layout, cap).unwrap();
    let mut field = zero(domain.layout());
    for (component, mean) in field.iter_mut().zip([3.0, 4.0, 12.0]) {
        component[0] = Complex64::new(mean, 0.0);
    }
    let report = work.sample(slices(&field)).unwrap();
    assert_eq!(report.velocity_maximum.value, 13.0);
    assert_eq!(report.velocity_maximum.ties, layout.real_len());
    assert_eq!(report.velocity_maximum.position, [0.0; 3]);
    assert_eq!(report.vorticity_maximum.value, 0.0);
    field[0][0] = Complex64::new(f64::MAX, 0.0);
    field[1][0] = field[0][0];
    assert_eq!(
        work.sample(slices(&field)).unwrap_err(),
        SolverError::ArithmeticResolutionLimited
    );
    let field = zero(domain.layout());
    assert_eq!(
        work.sample(slices(&field)).unwrap().velocity_maximum.value,
        0.0
    );
}
