//! Shared independent Fourier fixture comparison over explicitly listed modes.
use nsbu_solver::{domain::Layout, Complex64};

pub fn compare(layout: Layout, output: &[Vec<Complex64>; 3], fixture: &str, tolerance: f64) {
    for line in fixture.lines() {
        let columns: Vec<_> = line.split('\t').collect();
        let mode = std::array::from_fn(|i| columns[i].parse::<isize>().unwrap());
        let (index, conjugate) = layout.locate(mode).unwrap();
        assert!(!conjugate);
        for (component, values) in output.iter().enumerate() {
            let expected = Complex64::new(
                columns[3 + 2 * component].parse().unwrap(),
                columns[4 + 2 * component].parse().unwrap(),
            );
            assert!((values[index] - expected).l1_norm() < tolerance * (1.0 + expected.l1_norm()));
        }
    }
}
