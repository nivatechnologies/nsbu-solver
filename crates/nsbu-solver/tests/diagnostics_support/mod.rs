//! Analytical Fourier-mode fixtures, independent of production transforms and products.
use nsbu_solver::{domain::Layout, Complex64};

pub fn zero(layout: Layout) -> [Vec<Complex64>; 3] {
    std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); layout.half_len()])
}
pub fn slices(field: &[Vec<Complex64>; 3]) -> [&[Complex64]; 3] {
    std::array::from_fn(|axis| field[axis].as_slice())
}
pub fn mode(
    layout: Layout,
    field: &mut [Vec<Complex64>; 3],
    mode: [isize; 3],
    values: [Complex64; 3],
) {
    for (mode, values) in [(mode, values), (mode.map(|m| -m), values.map(|v| v.conj()))] {
        let (index, conjugate) = layout.locate(mode).unwrap();
        for (axis, value) in values.into_iter().enumerate() {
            field[axis][index] = if conjugate { value.conj() } else { value };
        }
    }
}
pub fn close(actual: f64, expected: f64) {
    assert!(
        (actual - expected).abs() <= 16.0 * f64::EPSILON * (1.0 + expected.abs()),
        "{actual:e} vs {expected:e}"
    );
}
