//! Complete-band physical errors, tensor multiplicity, curl signs and reusable failures.
use nsbu_solver::{
    diagnostics::physical::{PhysicalComparisonWorkspace, PhysicalField, PhysicalQuantity},
    domain::{Domain, Layout},
    Complex64,
};

fn domain(n: usize) -> Domain {
    Domain::new([n; 3], [2.0, 3.0, 4.0], 1.0).unwrap()
}
fn zero(domain: Domain) -> Vec<Complex64> {
    vec![Complex64::new(0.0, 0.0); domain.layout().half_len()]
}
fn fields(domain: Domain) -> [Vec<Complex64>; 3] {
    std::array::from_fn(|_| zero(domain))
}
fn view(fields: &[Vec<Complex64>; 3]) -> PhysicalField<'_> {
    PhysicalField::Vector(fields.each_ref().map(Vec::as_slice))
}
fn rms(quantity: PhysicalQuantity) -> f64 {
    let k = [
        3.0 * std::f64::consts::PI,
        -4.0 * std::f64::consts::PI / 3.0,
        std::f64::consts::PI / 2.0,
    ];
    let a = [0.4_f64, 0.5, -0.6];
    let magnitude = a[0].hypot(a[1]).hypot(a[2]);
    let wave = k[0].hypot(k[1]).hypot(k[2]);
    let amplitude = match quantity {
        PhysicalQuantity::Scalar => 1.0,
        PhysicalQuantity::ScalarGradient => wave,
        PhysicalQuantity::Vector => magnitude,
        PhysicalQuantity::Gradient => magnitude * wave,
        PhysicalQuantity::Hessian => magnitude * wave * wave,
        PhysicalQuantity::Vorticity => (k[1] * a[2] - k[2] * a[1])
            .hypot(k[2] * a[0] - k[0] * a[2])
            .hypot(k[0] * a[1] - k[1] * a[0]),
    };
    amplitude / 2.0_f64.sqrt()
}

#[test]
fn all_quantities_retain_fine_only_modes_and_full_derivative_multiplicity() {
    let left = domain(4);
    let right = domain(8);
    let samples = Layout::new([12; 3]).unwrap();
    let coarse = fields(left);
    let mut fine = fields(right);
    let index = right.layout().locate([3, -2, 1]).unwrap().0;
    for (component, amplitude) in fine.iter_mut().zip([0.4, 0.5, -0.6]) {
        component[index].re = amplitude / 2.0;
    }
    let before = fine.clone();
    let mut pressure = zero(right);
    pressure[index].re = 0.5;
    let mut workspace = PhysicalComparisonWorkspace::new(left, right, samples, 1 << 20).unwrap();
    for quantity in [
        PhysicalQuantity::Scalar,
        PhysicalQuantity::ScalarGradient,
        PhysicalQuantity::Vector,
        PhysicalQuantity::Gradient,
        PhysicalQuantity::Hessian,
        PhysicalQuantity::Vorticity,
    ] {
        let (a, b) = if matches!(
            quantity,
            PhysicalQuantity::Scalar | PhysicalQuantity::ScalarGradient
        ) {
            (
                PhysicalField::Scalar(&coarse[0]),
                PhysicalField::Scalar(&pressure),
            )
        } else {
            (view(&coarse), view(&fine))
        };
        let result = workspace.compare(a, b, quantity, 0.1).unwrap();
        let error = result.global();
        assert!(
            (error.rms_error - rms(quantity)).abs() < 1e-12,
            "{quantity:?}: {error:?}"
        );
        assert_eq!(error.components, quantity.components());
        assert_eq!(error.samples, samples.real_len());
        assert_eq!(error.relative_floor, 0.1);
        assert_eq!(error.peak_relative_error, 1.0);
        assert_eq!(result.quantity(), quantity);
        assert_eq!(result.domains(), [left, right]);
        assert_eq!(result.sample_layout(), samples);
        assert_eq!(result.error_magnitudes(), result.reference_magnitudes());
        assert_eq!(
            result.scalar_transforms(),
            if quantity == PhysicalQuantity::Vorticity {
                12
            } else {
                2 * quantity.components()
            }
        );
    }
    assert_eq!(fine, before);
}

#[test]
fn differences_are_of_fields_and_preserve_means_rather_than_subtracting_norms() {
    let domain = domain(4);
    let mut left = fields(domain);
    let mut right = fields(domain);
    left[0][0].re = 3.0;
    left[1][0].re = 4.0;
    right[0][0].re = -3.0;
    right[1][0].re = -4.0;
    let mut workspace =
        PhysicalComparisonWorkspace::new(domain, domain, domain.layout(), 1 << 20).unwrap();
    let result = workspace
        .compare(view(&left), view(&right), PhysicalQuantity::Vector, 0.5)
        .unwrap();
    assert_eq!(result.global().rms_error, 10.0);
    assert_eq!(result.global().reference_peak, 5.0);
    assert_eq!(result.global().peak_relative_error, 2.0);
    let pressure = workspace
        .compare(
            PhysicalField::Scalar(&left[0]),
            PhysicalField::Scalar(&right[0]),
            PhysicalQuantity::Scalar,
            0.5,
        )
        .unwrap();
    assert_eq!(pressure.global().peak_error, 6.0);
}

#[test]
fn incompatible_shapes_and_failed_late_components_cannot_produce_partial_reports() {
    let domain = domain(4);
    let good = fields(domain);
    let mut bad = good.clone();
    bad[2][0].re = f64::NAN;
    let mut workspace =
        PhysicalComparisonWorkspace::new(domain, domain, domain.layout(), 1 << 20).unwrap();
    assert!(workspace
        .compare(view(&good), view(&bad), PhysicalQuantity::Hessian, 1.0)
        .is_err());
    assert!(workspace
        .compare(
            view(&good),
            PhysicalField::Scalar(&good[0]),
            PhysicalQuantity::Vector,
            1.0
        )
        .is_err());
    assert!(workspace
        .compare(view(&good), view(&good), PhysicalQuantity::Scalar, 1.0)
        .is_err());
    assert!(workspace
        .compare(view(&good), view(&good), PhysicalQuantity::Vector, f64::NAN)
        .is_err());
    let result = workspace
        .compare(view(&good), view(&good), PhysicalQuantity::Hessian, 1.0)
        .unwrap();
    assert_eq!(result.global().rms_error, 0.0);
    assert_eq!(result.global().reference_peak, 0.0);
    assert_eq!(good, fields(domain));
}

#[test]
fn aggregate_preflight_rejects_caps_and_incompatible_domain_or_sample_grids() {
    let small = domain(4);
    let fine = domain(8);
    let samples = Layout::new([12; 3]).unwrap();
    let bytes = PhysicalComparisonWorkspace::reservation(small, fine, samples).unwrap();
    assert!(PhysicalComparisonWorkspace::new(small, fine, samples, bytes - 1).is_err());
    assert!(PhysicalComparisonWorkspace::new(small, fine, samples, bytes).is_ok());
    assert!(PhysicalComparisonWorkspace::reservation(fine, small, samples).is_err());
    assert!(PhysicalComparisonWorkspace::reservation(small, fine, small.layout()).is_err());
    let other = Domain::new([8; 3], [1.0; 3], 1.0).unwrap();
    assert!(PhysicalComparisonWorkspace::reservation(small, other, samples).is_err());
}
