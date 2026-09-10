//! Sampling refinement must examine force information outside the retained velocity band.
mod diagnostics_support;
use diagnostics_support::{close, mode, slices, zero};
use nsbu_solver::{
    diagnostics::comparison::ComparisonPlan,
    domain::Domain,
    spectral::{transfer, FftPlan},
    Complex64,
};

fn sampled_force(n: usize) -> (Domain, [Vec<Complex64>; 3]) {
    let domain = Domain::new([n; 3], [1.0; 3], 1.0).unwrap();
    let layout = domain.layout();
    let (fft, mut scratch) = FftPlan::new(layout, FftPlan::reservation(layout).unwrap()).unwrap();
    let real: Vec<f64> = (0..layout.real_len())
        .map(|index| {
            let x = (index / (n * n)) as f64 / n as f64;
            (5.0 * std::f64::consts::TAU * x).cos()
        })
        .collect();
    let mut raw = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    fft.forward(&real, &mut raw, &mut scratch).unwrap();
    let mut field = zero(layout);
    transfer(layout, layout, &raw, &mut field[0]).unwrap();
    (domain, field)
}

#[test]
fn cropped_force_agreement_can_hide_aliasing_and_missing_high_modes() {
    let (coarse, force4) = sampled_force(4);
    let (middle, force8) = sampled_force(8);
    let (fine, force12) = sampled_force(12);
    let mut retained8 = zero(coarse.layout());
    let mut retained12 = zero(coarse.layout());
    transfer(
        middle.layout(),
        coarse.layout(),
        &force8[0],
        &mut retained8[0],
    )
    .unwrap();
    transfer(
        fine.layout(),
        coarse.layout(),
        &force12[0],
        &mut retained12[0],
    )
    .unwrap();
    let retained = ComparisonPlan::new(coarse, coarse).unwrap();
    let cropped_gap = retained
        .compare(slices(&retained8), slices(&retained12))
        .unwrap();
    assert!(cropped_gap.full.l2 < 1e-14);
    close(
        retained
            .compare(slices(&force4), slices(&retained8))
            .unwrap()
            .full
            .l2,
        0.5_f64.sqrt(),
    );
    let full = ComparisonPlan::new(middle, fine)
        .unwrap()
        .compare(slices(&force8), slices(&force12))
        .unwrap();
    close(full.full.l2, 1.0);
    close(full.common.l2, 0.5_f64.sqrt());
    close(full.newly_resolved.l2, 0.5_f64.sqrt());
    let mut exact = zero(fine.layout());
    let z = Complex64::new(0.0, 0.0);
    mode(
        fine.layout(),
        &mut exact,
        [5, 0, 0],
        [Complex64::new(0.5, 0.0), z, z],
    );
    let resolved = ComparisonPlan::new(fine, fine)
        .unwrap()
        .compare(slices(&force12), slices(&exact))
        .unwrap();
    assert!(resolved.full.h1 < 1e-12);
}
