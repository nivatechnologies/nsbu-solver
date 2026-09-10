//! Finite spectra can still exceed derivative arithmetic; diagnostics must refuse them.
mod diagnostics_support;
use diagnostics_support::{close, mode, slices, zero};
use nsbu_solver::{
    diagnostics::{sampling::SamplingWorkspace, tails::TailPlan},
    domain::Domain,
    Complex64, SolverError,
};

#[test]
fn unresolved_wave_numbers_and_derivatives_never_become_zero_diagnostics() {
    for (length, amplitude, expected) in [
        (1e-310, 1.0, SolverError::InvalidSpectrum),
        (
            0.25,
            f64::MAX / 16.0,
            SolverError::ArithmeticResolutionLimited,
        ),
        (
            0.25,
            f64::MAX / 32.0,
            SolverError::ArithmeticResolutionLimited,
        ),
    ] {
        let domain = Domain::new([4; 3], [length, 1.0, 1.0], 1.0).unwrap();
        let layout = domain.layout();
        let mut field = zero(layout);
        mode(
            layout,
            &mut field,
            [1, 0, 0],
            [
                Complex64::new(0.0, 0.0),
                Complex64::new(amplitude, 0.0),
                Complex64::new(0.0, 0.0),
            ],
        );
        let tails = TailPlan::new(domain, [1; 3]).unwrap();
        assert_eq!(tails.measure(slices(&field)).unwrap_err(), expected);
        let cap = SamplingWorkspace::reservation(domain, layout).unwrap();
        let mut sampler = SamplingWorkspace::new(domain, layout, cap).unwrap();
        assert_eq!(
            sampler.sample(slices(&field)).unwrap_err(),
            SolverError::InvalidSpectrum
        );
        if length == 0.25 {
            let restored = zero(layout);
            let report = sampler.sample(slices(&restored)).unwrap();
            close(report.velocity_maximum.value, 0.0);
            close(report.vorticity_maximum.value, 0.0);
        }
    }
}
