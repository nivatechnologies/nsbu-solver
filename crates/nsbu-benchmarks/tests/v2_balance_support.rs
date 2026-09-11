//! Independent helpers for exact-v2 reconstructed balance and Simpson tests.
use nsbu_benchmarks::{provider::V2Force, v2_experiment::probes::ProbeFields};
use nsbu_solver::{
    diagnostics::{balances, conservative::ConservativeWorkspace, quadrature::BalanceIntegral},
    integrators::forcing::PrescribedForce,
    spectral::transfer,
    Complex64,
};

/// Rebuild one balance with the fixed force profile and with force omitted.
pub fn direct_balance_with_omission(
    fields: &ProbeFields<'_>,
    samples: nsbu_solver::domain::Layout,
) -> (balances::BalanceSample, balances::BalanceSample) {
    let diagnostic = ConservativeWorkspace::diagnostic_domain(fields.domain).unwrap();
    let limits = V2Force::preflight(diagnostic, samples).unwrap();
    let mut provider = V2Force::new(diagnostic, samples, limits.storage_bytes).unwrap();
    let m = diagnostic.layout().half_len();
    let mut force: [Vec<Complex64>; 3] = std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); m]);
    provider
        .evaluate(
            fields.clock,
            limits,
            force.each_mut().map(Vec::as_mut_slice),
        )
        .unwrap();
    let actual = assemble(fields, force.each_ref().map(Vec::as_slice)).0;
    let zero: [Vec<Complex64>; 3] = std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); m]);
    let omitted = assemble(fields, zero.each_ref().map(Vec::as_slice)).0;
    (actual, omitted)
}

fn assemble(
    fields: &ProbeFields<'_>,
    force: [&[Complex64]; 3],
) -> (balances::BalanceSample, [Vec<Complex64>; 3]) {
    let diagnostic = ConservativeWorkspace::diagnostic_domain(fields.domain).unwrap();
    let m = diagnostic.layout().half_len();
    let mut products = ConservativeWorkspace::new(
        fields.domain,
        ConservativeWorkspace::reservation(fields.domain).unwrap(),
    )
    .unwrap();
    let mut conservative: [Vec<Complex64>; 3] =
        std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); m]);
    let mut pressure = vec![Complex64::new(0.0, 0.0); m];
    products
        .evaluate(
            fields.value,
            force,
            conservative.each_mut().map(Vec::as_mut_slice),
            &mut pressure,
        )
        .unwrap();
    let mut padded: [Vec<Complex64>; 3] =
        std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); m]);
    for (input, output) in fields.value.into_iter().zip(&mut padded) {
        transfer(fields.domain.layout(), diagnostic.layout(), input, output).unwrap();
    }
    let sample = balances::measure(
        diagnostic,
        padded.each_ref().map(Vec::as_slice),
        force,
        conservative.each_ref().map(Vec::as_slice),
    )
    .unwrap();
    (sample, conservative)
}

/// Reduce stored samples with explicit composite Simpson weights.
pub fn hand_simpson(
    clocks: &[nsbu_solver::domain::TickClock],
    samples: &[balances::BalanceSample],
) -> BalanceIntegral {
    assert_eq!(clocks.len(), samples.len());
    let quantum = 2.0_f64.powi(clocks[0].exponent());
    let mut energy_rhs = 0.0;
    let mut enstrophy_rhs = 0.0;
    for index in (0..clocks.len() - 2).step_by(2) {
        let duration = (clocks[index + 2].elapsed() - clocks[index].elapsed()) as f64 * quantum;
        let weights = [duration / 6.0, 2.0 * duration / 3.0, duration / 6.0];
        for offset in 0..3 {
            let sample = samples[index + offset];
            energy_rhs += weights[offset] * (sample.forcing_work - sample.energy_dissipation);
            enstrophy_rhs += weights[offset]
                * (sample.stretching + sample.vorticity_forcing - sample.enstrophy_dissipation);
        }
    }
    BalanceIntegral {
        energy_rhs,
        enstrophy_rhs,
        energy_defect: samples.last().unwrap().energy - samples[0].energy - energy_rhs,
        enstrophy_defect: samples.last().unwrap().enstrophy - samples[0].enstrophy - enstrophy_rhs,
    }
}

/// Compare every integral and endpoint-defect channel.
pub fn assert_integral(actual: BalanceIntegral, expected: BalanceIntegral, tolerance: f64) {
    for (a, b) in [
        (actual.energy_rhs, expected.energy_rhs),
        (actual.enstrophy_rhs, expected.enstrophy_rhs),
        (actual.energy_defect, expected.energy_defect),
        (actual.enstrophy_defect, expected.enstrophy_defect),
    ] {
        assert!((a - b).abs() <= tolerance.max(tolerance * b.abs()));
    }
}
