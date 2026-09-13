//! Exact quarter-period checks of independently validated modal derivatives.
//! These artificial modes test the operator, not PDE convergence.
#[path = "fixtures/local_modal_derivatives.rs"]
mod fixtures;

use fixtures::{ModalFixture, FIXTURES};
use nsbu_solver::{
    diagnostics::derivatives::{Derivative, DerivativeWorkspace},
    domain::{Domain, Layout},
    Complex64,
};

fn difference(domain: Domain, fixture: &ModalFixture, component: usize) -> Vec<Complex64> {
    let mut values = vec![Complex64::new(0.0, 0.0); domain.layout().half_len()];
    let left = fixture.left[component];
    let right = fixture.right[component];
    let coefficient = Complex64::new((right[0] - left[0]) as f64, (right[1] - left[1]) as f64);
    let (index, conjugated) = domain.layout().locate(fixture.mode).unwrap();
    assert!(!conjugated);
    values[index] = coefficient;
    // The half-spectrum stores the negative-z partner implicitly. On z=0,
    // a nonzero horizontal wave needs an explicit conjugate partner.
    if fixture.mode[2] == 0 && fixture.mode != [0; 3] {
        let partner = fixture.mode.map(|k| -k);
        values[domain.layout().locate(partner).unwrap().0] = coefficient.conj();
    }
    values
}

fn quarter_phase(mode: [isize; 3], index: usize) -> Option<(f64, f64)> {
    let point = [index / 256, (index / 16) % 16, index % 16];
    let phase = mode
        .into_iter()
        .zip(point)
        .map(|(k, x)| k * x as isize)
        .sum::<isize>();
    match phase.rem_euclid(16) {
        0 => Some((1.0, 0.0)),
        4 => Some((0.0, 1.0)),
        8 => Some((-1.0, 0.0)),
        12 => Some((0.0, -1.0)),
        _ => None,
    }
}

fn check_row(
    workspace: &mut DerivativeWorkspace,
    input: &[Complex64],
    mode: [isize; 3],
    row: [i64; 6],
) {
    let orders = [row[1], row[2], row[3]].map(|v| u8::try_from(v).unwrap());
    let result = workspace
        .sample(input, Derivative::new(orders).unwrap())
        .unwrap();
    let mut checked = 0;
    for (index, &actual) in result.values.iter().enumerate() {
        let Some((cos, sin)) = quarter_phase(mode, index) else {
            continue;
        };
        let multiplicity = if mode == [0; 3] { 1.0 } else { 2.0 };
        let expected = multiplicity * (row[4] as f64 * cos - row[5] as f64 * sin);
        assert!(
            (actual - expected).abs() < 1e-10,
            "mode={mode:?} row={row:?} index={index}: {actual} != {expected}"
        );
        checked += 1;
    }
    assert!(checked > 0);
}

#[test]
fn all_312_modal_entries_match_actual_derivative_samples_without_mutation() {
    let domain = Domain::new([8; 3], [std::f64::consts::TAU; 3], 1.0).unwrap();
    let mut workspace =
        DerivativeWorkspace::new(domain, Layout::new([16; 3]).unwrap(), 1 << 20).unwrap();
    let mut rows = 0;
    for fixture in &FIXTURES {
        for row in fixture.rows {
            let input = difference(domain, fixture, usize::try_from(row[0]).unwrap());
            let before = input.clone();
            check_row(&mut workspace, &input, fixture.mode, row);
            assert_eq!(input, before);
            rows += 1;
        }
    }
    assert_eq!(rows, 312);
}
