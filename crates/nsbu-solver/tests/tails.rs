//! Directional spectra retain cutoff-boundary modes and disclose overlapping subsets.
mod diagnostics_support;
use diagnostics_support::{close, mode, slices, zero};
use nsbu_solver::{diagnostics::tails::TailPlan, domain::Domain, Complex64, SolverError};

#[test]
fn directional_tail_norms_respect_physical_derivatives_and_overlap() {
    let domain = Domain::new([8; 3], [1.0; 3], 1.0).unwrap();
    let mut input = zero(domain.layout());
    let z = Complex64::new(0.0, 0.0);
    mode(
        domain.layout(),
        &mut input,
        [2, 0, 0],
        [z, Complex64::new(0.0, -0.5), z],
    );
    mode(
        domain.layout(),
        &mut input,
        [0, 2, 1],
        [Complex64::new(0.0, -1.0), z, z],
    );
    mode(
        domain.layout(),
        &mut input,
        [1, 1, 1],
        [Complex64::new(0.0, -0.5), Complex64::new(0.0, 0.5), z],
    );
    for (component, mean) in input.iter_mut().zip([10.0, 20.0, 30.0]) {
        component[0] = Complex64::new(mean, 0.0);
    }
    let tails = TailPlan::new(domain, [2; 3])
        .unwrap()
        .measure(slices(&input))
        .unwrap();
    assert_eq!(tails.cutoffs, [2; 3]);
    let k = std::f64::consts::TAU;
    close(tails.norms[0].l2, 0.5_f64.sqrt());
    close(tails.norms[0].h1, (0.5 + 2.0 * k * k).sqrt());
    close(tails.norms[0].vorticity_l2, 2.0_f64.sqrt() * k);
    close(tails.norms[1].l2, 2.0_f64.sqrt());
    close(tails.norms[1].h1, (2.0 + 10.0 * k * k).sqrt());
    close(tails.norms[1].vorticity_l2, 10.0_f64.sqrt() * k);
    assert_eq!(tails.norms[2].h1, 0.0);
    for norm in tails.norms {
        assert_eq!(norm.divergence_l2, 0.0);
    }
    let overlap = TailPlan::new(domain, [1; 3])
        .unwrap()
        .measure(slices(&input))
        .unwrap();
    close(overlap.norms[0].l2, 1.5_f64.sqrt());
    close(overlap.norms[1].l2, 3.0_f64.sqrt());
    close(overlap.norms[2].l2, 3.0_f64.sqrt());
}

#[test]
fn invalid_tail_thresholds_and_incomplete_spectra_are_refused() {
    let domain = Domain::new([8; 3], [1.0; 3], 1.0).unwrap();
    for cutoffs in [[0, 1, 1], [1, 4, 1], [1, 1, 5]] {
        assert_eq!(
            TailPlan::new(domain, cutoffs).unwrap_err(),
            SolverError::InvalidIndex
        );
    }
    let plan = TailPlan::new(domain, [2; 3]).unwrap();
    let mut input = zero(domain.layout());
    input[1].pop();
    assert_eq!(
        plan.measure(slices(&input)).unwrap_err(),
        SolverError::InvalidPayload
    );
    let mut input = zero(domain.layout());
    input[2][domain.layout().index([4, 0, 0]).unwrap()] = Complex64::new(1.0, 0.0);
    assert_eq!(
        plan.measure(slices(&input)).unwrap_err(),
        SolverError::InvalidSpectrum
    );
}
