//! Local derivative errors retain the global/collar partition and full tensor magnitudes.
use nsbu_benchmarks::regions::{RegionalTensorErrors, SpatialRegion};
use nsbu_solver::{
    diagnostics::local::SampledError,
    domain::{Layout, TickClock},
};

#[test]
fn hessian_error_in_collar_is_visible_in_global_and_collar_reports() {
    let clock = TickClock::restore(-20, 8192, 4096, 4096).unwrap();
    let layout = Layout::new([4; 3]).unwrap();
    let mut errors = RegionalTensorErrors::<27>::new(clock, layout, 128, 64, 0.5).unwrap();
    let mut active = 0;
    while let Some(point) = errors.next_point() {
        let mut actual = [0.0; 27];
        if point == [0.25, 0.25, 0.0] {
            actual[1] = 3.0;
            actual[3] = 3.0;
            active += 1;
        }
        errors.push(actual, [0.0; 27]).unwrap();
    }
    assert_eq!(active, 1);
    let report = errors.report().unwrap();
    assert_eq!(report.components, 27);
    assert!(report.grid_complete);
    let SampledError::Measured(global) = report.global else {
        panic!("missing global")
    };
    assert!((global.rms_error - (18.0_f64 / 64.0).sqrt()).abs() < 2e-15);
    for (region, sample) in report.regions {
        let SampledError::Measured(local) = sample else {
            panic!("fixture region unsampled")
        };
        let expected = if region == SpatialRegion::Collar {
            18.0_f64.sqrt()
        } else {
            0.0
        };
        assert_eq!(local.components, 27);
        assert!((local.peak_error - expected).abs() < 2e-15);
    }
    assert_eq!(report.root_work_charged, 64 * 128);
    assert!(RegionalTensorErrors::<0>::new(clock, layout, 128, 64, 1.0).is_err());
}
