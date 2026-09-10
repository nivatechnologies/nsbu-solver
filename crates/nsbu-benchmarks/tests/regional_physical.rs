//! Complete physical comparisons share every point with the v2 regional report.
use nsbu_benchmarks::regions::physical;
use nsbu_solver::{
    diagnostics::{
        local::SampledError,
        physical::{PhysicalComparisonWorkspace, PhysicalField, PhysicalQuantity},
    },
    domain::{Domain, TickClock},
    Complex64,
};

#[test]
fn every_quantity_preserves_global_statistics_across_complete_geometric_partition() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let clock = TickClock::restore(-20, 8192, 4096, 4096).unwrap();
    let zero = vec![Complex64::new(0.0, 0.0); domain.layout().half_len()];
    let mut field = zero.clone();
    field[0].re = 2.0;
    for mode in [[1, 0, 0], [-1, 0, 0]] {
        field[domain.layout().locate(mode).unwrap().0].re = 0.5;
    }
    let mut workspace =
        PhysicalComparisonWorkspace::new(domain, domain, domain.layout(), 1 << 20).unwrap();
    for quantity in [
        PhysicalQuantity::Scalar,
        PhysicalQuantity::ScalarGradient,
        PhysicalQuantity::Vector,
        PhysicalQuantity::Gradient,
        PhysicalQuantity::Hessian,
        PhysicalQuantity::Vorticity,
    ] {
        let (left, right) = if matches!(
            quantity,
            PhysicalQuantity::Scalar | PhysicalQuantity::ScalarGradient
        ) {
            (PhysicalField::Scalar(&field), PhysicalField::Scalar(&zero))
        } else {
            (
                PhysicalField::Vector([&field; 3]),
                PhysicalField::Vector([&zero; 3]),
            )
        };
        let result = workspace.compare(left, right, quantity, 0.5).unwrap();
        let regional = physical::measure(&result, clock, 128, 64).unwrap();
        assert_eq!(regional.global, SampledError::Measured(result.global()));
        assert_eq!(regional.components, quantity.components());
        assert!(regional.grid_complete);
        let count: usize = regional
            .regions
            .into_iter()
            .map(|(_, sample)| match sample {
                SampledError::Measured(value) => value.samples,
                SampledError::NoSamples => 0,
            })
            .sum();
        assert_eq!(count, 64);
        assert_eq!(regional.root_work_charged, 64 * 128);
        assert!(physical::measure(&result, clock, 0, 64).is_err());
        assert!(physical::measure(&result, clock, 128, 63).is_err());
        assert!(physical::measure(
            &result,
            TickClock::from_rest(-20, 1 << 20).unwrap(),
            128,
            64
        )
        .is_err());
    }
}

#[test]
fn v2_region_adapter_refuses_a_different_physical_problem_geometry() {
    let domain = Domain::new([4; 3], [2.0; 3], 1.0).unwrap();
    let zero = vec![Complex64::new(0.0, 0.0); domain.layout().half_len()];
    let mut work =
        PhysicalComparisonWorkspace::new(domain, domain, domain.layout(), 1 << 20).unwrap();
    let result = work
        .compare(
            PhysicalField::Scalar(&zero),
            PhysicalField::Scalar(&zero),
            PhysicalQuantity::Scalar,
            1.0,
        )
        .unwrap();
    let clock = TickClock::restore(-20, 8192, 4096, 4096).unwrap();
    assert!(physical::measure(&result, clock, 128, 64).is_err());
}
