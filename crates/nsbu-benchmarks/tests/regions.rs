//! Independent high-precision volume fractions and masks unaffected by rounded cutoff plateaus.
use nsbu_benchmarks::{
    regions::{classify, CoveragePlan, CoverageStatus, NominalRegion, SpatialRegion, StartupPhase},
    scalar::smooth_step,
    time::BenchmarkTime,
    BenchmarkError,
};
use nsbu_solver::domain::TickClock;

fn clock(remaining: u128) -> TickClock {
    TickClock::restore(-20, 8192, 8192 - remaining, remaining).unwrap()
}

#[test]
fn volume_fractions_match_independent_high_precision_quadrature() {
    for line in include_str!("fixtures/region-coverage.tsv").lines() {
        let values: Vec<&str> = line.split('\t').collect();
        let remaining = values[0].parse().unwrap();
        let region = if values[1] == "0" {
            NominalRegion::CORE
        } else {
            NominalRegion::ANNULUS
        };
        let panels: usize = values[2].parse().unwrap();
        let fraction: f64 = values[3].parse().unwrap();
        let change: f64 = values[4].parse().unwrap();
        let report = CoveragePlan::new(panels, 3 * panels + 2)
            .unwrap()
            .evaluate(clock(remaining), region)
            .unwrap();
        assert_eq!(report.status, CoverageStatus::Nonempty);
        assert_eq!(report.evaluations, 3 * panels + 2);
        assert_eq!(report.panels, 2 * panels);
        assert!((report.fraction - fraction).abs() < 4e-14);
        assert!((report.refinement_change - change).abs() < 5e-15);
    }
}

#[test]
fn geometric_emptiness_and_late_time_coverage_do_not_depend_on_underflow() {
    let plan = CoveragePlan::new(4, 14).unwrap();
    let empty = plan
        .evaluate(clock(8192), NominalRegion::new(20.0, 30.0).unwrap())
        .unwrap();
    assert_eq!(empty.status, CoverageStatus::RegionEmpty);
    assert_eq!(empty.fraction, 0.0);
    assert_eq!(empty.refinement_change, 0.0);
    let late = TickClock::restore(-134, 1 << 127, (1 << 127) - 1, 1).unwrap();
    for region in [
        NominalRegion::CORE,
        NominalRegion::ANNULUS,
        NominalRegion::new(0.0, f64::from_bits(1)).unwrap(),
    ] {
        let report = plan.evaluate(late, region).unwrap();
        assert_eq!(report.status, CoverageStatus::Nonempty);
        assert_eq!(report.fraction, 1.0);
    }
    let coarse = CoveragePlan::new(64, 194)
        .unwrap()
        .evaluate(clock(8192), NominalRegion::ANNULUS)
        .unwrap();
    let fine = CoveragePlan::new(128, 386)
        .unwrap()
        .evaluate(clock(8192), NominalRegion::ANNULUS)
        .unwrap();
    let ratio = coarse.refinement_change / fine.refinement_change;
    assert!(ratio > 15.0);
    assert!(ratio < 17.0);
}

#[test]
fn spherical_masks_do_not_use_the_rounded_smooth_step_value() {
    for (x, rounded) in [(0.301, 1.0), (0.419999, 0.0)] {
        let cutoff = smooth_step((441.0 / 2500.0 - x * x) / (54.0 / 625.0))
            .unwrap()
            .0;
        assert_eq!(cutoff, rounded);
        let report = classify([x, 0.0, 0.0], clock(4096), 128).unwrap();
        assert_eq!(report.spatial, SpatialRegion::Collar);
        assert!(report.root.is_none());
        assert_eq!(report.startup, StartupPhase::Active);
    }
    assert_eq!(
        classify([0.43, 0.0, 0.0], clock(4096), 128)
            .unwrap()
            .spatial,
        SpatialRegion::Exterior
    );
    assert_eq!(
        classify([0.0; 3], clock(8192), 128).unwrap().spatial,
        SpatialRegion::Core
    );
    assert_eq!(
        classify([0.125, 0.0, 0.0], clock(8192), 128)
            .unwrap()
            .spatial,
        SpatialRegion::Annulus
    );
    assert_eq!(
        classify([0.1875, 0.0, 0.0], clock(2048), 128)
            .unwrap()
            .spatial,
        SpatialRegion::InteriorOutsideNominal
    );
    let z = 0.6 * (1.0_f64 / 256.0 / (1.0 - 0.6 * 0.6)).powf(3.0 / 8.0);
    let report = classify([0.0, 0.0, z], clock(4096), 128).unwrap();
    assert_eq!(report.spatial, SpatialRegion::InteriorOutsideNominal);
    assert!(report.root.unwrap().iterations > 0);
}

#[test]
fn startup_phase_uses_integer_ticks_even_when_binary64_rounds_onto_the_boundary() {
    let target = 1 << 127;
    let elapsed = (1 << 125) - 1;
    let just_before = TickClock::restore(-134, target, elapsed, target - elapsed).unwrap();
    assert_eq!(
        BenchmarkTime::new(just_before).unwrap().elapsed(),
        1.0 / 512.0
    );
    assert_eq!(
        classify([0.49, 0.0, 0.0], just_before, 128)
            .unwrap()
            .startup,
        StartupPhase::Ramping
    );
    assert_eq!(
        classify([0.0; 3], clock(8192), 128).unwrap().startup,
        StartupPhase::Rest
    );
    assert_eq!(
        classify([0.0; 3], clock(6144), 128).unwrap().startup,
        StartupPhase::Active
    );
}

#[test]
fn invalid_geometry_work_budgets_and_clocks_are_refused() {
    for (low, high) in [
        (f64::NAN, 1.0),
        (0.0, f64::INFINITY),
        (-1.0, 1.0),
        (1.0, 1.0),
        (2.0, 1.0),
    ] {
        assert_eq!(
            NominalRegion::new(low, high).unwrap_err(),
            BenchmarkError::InvalidInput
        );
    }
    for panels in [0, 1, 3] {
        assert_eq!(
            CoveragePlan::new(panels, 100).unwrap_err(),
            BenchmarkError::InvalidInput
        );
    }
    assert_eq!(
        CoveragePlan::new(4, 13).unwrap_err(),
        BenchmarkError::DiagnosticWorkExceeded
    );
    assert_eq!(
        CoveragePlan::new((1 << 20) + 2, usize::MAX).unwrap_err(),
        BenchmarkError::DiagnosticWorkExceeded
    );
    let wrong = TickClock::from_rest(-10, 64).unwrap();
    assert_eq!(
        CoveragePlan::new(4, 14)
            .unwrap()
            .evaluate(wrong, NominalRegion::CORE)
            .unwrap_err(),
        BenchmarkError::ClockIdentity
    );
    assert_eq!(
        classify([0.0; 3], wrong, 128).unwrap_err(),
        BenchmarkError::ClockIdentity
    );
    assert_eq!(
        classify([f64::NAN, 0.0, 0.0], clock(4096), 128).unwrap_err(),
        BenchmarkError::InvalidInput
    );
    for budget in [0, 129] {
        assert_eq!(
            classify([0.49, 0.0, 0.0], clock(4096), budget).unwrap_err(),
            BenchmarkError::InvalidInput
        );
    }
    assert_eq!(
        classify([0.0, 0.0, 0.1], clock(4096), 1).unwrap_err(),
        BenchmarkError::RootWorkExhausted
    );
}

#[test]
fn declared_panel_and_similarity_boundaries_are_included_correctly() {
    assert!(CoveragePlan::new(1 << 20, 3 * (1 << 20) + 2).is_ok());
    let minimal = CoveragePlan::new(2, 8)
        .unwrap()
        .evaluate(clock(4096), NominalRegion::CORE)
        .unwrap();
    assert_eq!(minimal.panels, 4);
    assert_eq!(minimal.evaluations, 8);
    assert_eq!(
        classify([0.0; 3], clock(4096), 1).unwrap().spatial,
        SpatialRegion::Core
    );
    assert_eq!(
        classify([0.0625, 0.0, 0.0], clock(4096), 128)
            .unwrap()
            .spatial,
        SpatialRegion::Core
    );
    assert_eq!(
        classify([0.25, 0.0, 0.0], clock(4096), 128)
            .unwrap()
            .spatial,
        SpatialRegion::Annulus
    );
    assert_eq!(
        classify([0.0, 0.0, 0.0625], clock(3072), 128)
            .unwrap()
            .spatial,
        SpatialRegion::Core
    );
    assert_eq!(
        classify([0.3, 0.0, 0.0], clock(4096), 128).unwrap().spatial,
        SpatialRegion::InteriorOutsideNominal
    );
    let y = (441.0_f64 / 2500.0 - 0.42_f64.powi(2)).sqrt();
    assert_eq!(
        classify([0.42, y, 0.0], clock(4096), 128).unwrap().spatial,
        SpatialRegion::Exterior
    );
}

#[test]
fn radial_masks_are_invariant_under_transverse_rotations() {
    for (point, expected) in [
        ([0.03, 0.04, 0.0], SpatialRegion::Core),
        ([0.125, 0.0, 0.0], SpatialRegion::Annulus),
        ([0.3, 0.0, 0.0], SpatialRegion::InteriorOutsideNominal),
        ([0.301, 0.0, 0.0], SpatialRegion::Collar),
        ([0.43, 0.0, 0.0], SpatialRegion::Exterior),
        ([0.0, 0.0, 0.301], SpatialRegion::Collar),
        ([0.0, 0.0, -0.43], SpatialRegion::Exterior),
    ] {
        let [x, y, z] = point;
        for rotated in [[x, y, z], [-y, x, z], [-x, -y, z]] {
            assert_eq!(
                classify(rotated, clock(4096), 128).unwrap().spatial,
                expected
            );
        }
    }
}
