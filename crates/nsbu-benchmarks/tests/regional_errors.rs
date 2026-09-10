//! Global and local errors share a declared lattice, with bounded failed geometry attempts.
use nsbu_benchmarks::{
    regions::{RegionalError, RegionalErrors, SpatialRegion},
    BenchmarkError,
};
use nsbu_solver::{
    diagnostics::local::{LocalError, SampledError},
    domain::{Layout, TickClock},
    SolverError,
};

fn clock() -> TickClock {
    TickClock::restore(-20, 8192, 4096, 4096).unwrap()
}

fn measured(value: SampledError) -> Option<LocalError> {
    match value {
        SampledError::NoSamples => None,
        SampledError::Measured(error) => Some(error),
    }
}

#[test]
fn integer_lattice_partition_preserves_global_and_collar_errors() {
    let layout = Layout::new([4; 3]).unwrap();
    let mut errors = RegionalErrors::new(clock(), layout, 128, 65, 1.0).unwrap();
    assert_eq!(measured(errors.report().unwrap().global), None);
    assert!(!errors.report().unwrap().grid_complete);
    for value in 1..=64 {
        errors.push([f64::from(value), 0.0, 0.0], [0.0; 3]).unwrap();
    }
    let report = errors.report().unwrap();
    assert!(report.grid_complete);
    assert_eq!(report.clock, clock());
    assert_eq!(report.dimensions, [4; 3]);
    assert_eq!(report.root_work_charged, 64 * 128);
    let global = measured(report.global).unwrap();
    assert_eq!(global.samples, 64);
    assert!((global.rms_error - 1397.5_f64.sqrt()).abs() < 2e-14);
    assert_eq!(global.peak_error, 64.0);
    for ((label, samples), (expected, count, sum_squares, peak)) in
        report.regions.into_iter().zip([
            (SpatialRegion::Core, 1, 1.0, 1.0),
            (SpatialRegion::Annulus, 4, 2884.0, 49.0),
            (SpatialRegion::InteriorOutsideNominal, 2, 20.0, 4.0),
            (SpatialRegion::Collar, 12, 14292.0, 61.0),
            (SpatialRegion::Exterior, 45, 72243.0, 64.0),
        ])
    {
        assert_eq!(label, expected);
        let local = measured(samples).unwrap();
        assert_eq!(local.samples, count);
        assert!((local.rms_error - (sum_squares / count as f64).sqrt()).abs() < 2e-14);
        assert_eq!(local.peak_error, peak);
        assert_eq!(local.peak_relative_error, peak);
    }
    assert_eq!(
        errors.push([0.0; 3], [0.0; 3]).unwrap_err(),
        RegionalError::Measurement(SolverError::ResourceLimit)
    );
    assert_eq!(errors.report().unwrap().global, report.global);
    assert_eq!(errors.report().unwrap().root_work_charged, 8192);
}

#[test]
fn failed_samples_preserve_position_and_failed_roots_consume_finite_work() {
    let layout = Layout::new([4; 3]).unwrap();
    let mut errors = RegionalErrors::new(clock(), layout, 1, 64, 1.0).unwrap();
    assert_eq!(
        errors.push([f64::NAN, 0.0, 0.0], [0.0; 3]).unwrap_err(),
        RegionalError::Measurement(SolverError::InvalidSpectrum)
    );
    assert_eq!(errors.report().unwrap().root_work_charged, 0);
    errors.push([3.0, 4.0, 0.0], [0.0; 3]).unwrap();
    let before = errors.report().unwrap();
    assert_eq!(measured(before.global).unwrap().samples, 1);
    for _ in 1..64 {
        assert_eq!(
            errors.push([0.0; 3], [0.0; 3]).unwrap_err(),
            RegionalError::Geometry(BenchmarkError::RootWorkExhausted)
        );
    }
    let after = errors.report().unwrap();
    assert_eq!(after.root_work_charged, 64);
    assert_eq!(after.global, before.global);
    assert_eq!(after.regions, before.regions);
    assert!(!after.grid_complete);
    assert_eq!(
        errors.push([0.0; 3], [0.0; 3]).unwrap_err(),
        RegionalError::Geometry(BenchmarkError::DiagnosticWorkExceeded)
    );
}

#[test]
fn invalid_identity_storage_and_work_are_refused_before_sampling() {
    let layout = Layout::new([4; 3]).unwrap();
    for (budget, attempts, floor, expected) in [
        (
            0,
            64,
            1.0,
            RegionalError::Geometry(BenchmarkError::InvalidInput),
        ),
        (
            129,
            64,
            1.0,
            RegionalError::Geometry(BenchmarkError::InvalidInput),
        ),
        (
            128,
            63,
            1.0,
            RegionalError::Geometry(BenchmarkError::DiagnosticWorkExceeded),
        ),
        (
            128,
            usize::MAX,
            1.0,
            RegionalError::Geometry(BenchmarkError::DiagnosticWorkExceeded),
        ),
        (
            128,
            64,
            0.0,
            RegionalError::Measurement(SolverError::InvalidPayload),
        ),
    ] {
        assert_eq!(
            RegionalErrors::new(clock(), layout, budget, attempts, floor).unwrap_err(),
            expected
        );
    }
    let other = TickClock::restore(-19, 8192, 4096, 4096).unwrap();
    assert_eq!(
        RegionalErrors::new(other, layout, 128, 64, 1.0).unwrap_err(),
        RegionalError::Geometry(BenchmarkError::ClockIdentity)
    );
}

#[test]
fn anisotropic_sample_coordinates_are_exposed_without_periodic_relabeling() {
    let layout = Layout::new([4, 6, 8]).unwrap();
    let mut errors = RegionalErrors::new(clock(), layout, 128, 192, 1.0).unwrap();
    for x in 0..4 {
        for y in 0..6 {
            for z in 0..8 {
                let point = [f64::from(x) / 4.0, f64::from(y) / 6.0, f64::from(z) / 8.0];
                assert_eq!(errors.next_point(), Some(point));
                errors.push(point, point).unwrap();
            }
        }
    }
    assert_eq!(errors.next_point(), None);
    let report = errors.report().unwrap();
    assert_eq!(report.dimensions, [4, 6, 8]);
    assert!(report.grid_complete);
    assert_eq!(measured(report.global).unwrap().rms_error, 0.0);
}
