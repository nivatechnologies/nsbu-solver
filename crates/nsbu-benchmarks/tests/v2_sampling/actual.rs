//! Actual-family report and witness controls.
use super::*;

#[inline(never)]
fn measured(maximum: SampleMaximum) -> Option<(Layout, usize, [usize; 3], f64)> {
    match maximum {
        SampleMaximum::NoSamples => None,
        SampleMaximum::Measured {
            layout,
            linear,
            index,
            value,
        } => Some((layout, linear, index, value)),
    }
}

#[test]
fn actual_family_retains_every_lattice_pair_metric_and_peak_witness() {
    assert_eq!(measured(SampleMaximum::NoSamples), None);
    let clocks = v2_family_support::clocks();
    let (family_plan, sampling_plan) = setup(&clocks, 5);
    assert_eq!(sampling_plan.sample_layouts(), layouts());
    assert_eq!(sampling_plan.relative_floors(), FLOORS);
    assert_eq!(
        sampling_plan.bounds().joint_storage_bytes,
        family_plan.bounds().storage_bytes + sampling_plan.bounds().storage_bytes
    );
    let mut family = V2Family::new(family_plan).unwrap();
    let mut sampling = SamplingWorkspace::new(sampling_plan).unwrap();
    assert!(matches!(
        sampling.measure(&family),
        Err(FamilyError::InvalidFamily)
    ));
    for clock in clocks {
        family.advance().unwrap().unwrap();
        let before = digest(&family);
        let report = sampling.measure(&family).unwrap();
        assert_eq!(digest(&family), before);
        assert_eq!(report.clock(), clock);
        assert_eq!(report.identity(), family_plan.identity());
        assert_eq!(report.sample_layouts(), layouts());
        assert_eq!(report.relative_floors(), FLOORS);
        for (quantity_index, quantity) in report.quantities().iter().enumerate() {
            assert_eq!(quantity.quantity(), QUANTITIES[quantity_index]);
            for pair in 0..5 {
                let metrics = quantity.pair(pair).unwrap();
                let extrema = quantity.extrema(pair).unwrap();
                for level in 0..3 {
                    let layout = layouts()[level];
                    let metric = metrics[level];
                    assert_eq!(metric.samples, layout.real_len());
                    assert_eq!(metric.components, quantity.quantity().components());
                    assert_eq!(metric.relative_floor, FLOORS[quantity_index]);
                    for (maximum, expected) in [
                        (extrema[level].error, metric.peak_error),
                        (extrema[level].relative_error, metric.peak_relative_error),
                        (extrema[level].reference, metric.reference_peak),
                    ] {
                        let found = measured(maximum);
                        assert!(found.is_some(), "complete physical layouts require samples");
                        let (actual, linear, index, value) =
                            found.unwrap_or((layout, usize::MAX, [usize::MAX; 3], f64::NAN));
                        assert_eq!(actual, layout);
                        assert_eq!(
                            linear,
                            (index[0] * layout.dimensions()[1] + index[1]) * layout.dimensions()[2]
                                + index[2]
                        );
                        assert_eq!(value, expected);
                    }
                    println!("tick={} quantity={:?} pair={pair} level={level} layout={:?} metric={metric:?} extrema={:?}",
                        clock.elapsed(), quantity.quantity(), layout.dimensions(), extrema[level]);
                }
                for change in quantity.changes(pair).unwrap() {
                    assert!(change.rms_error.is_finite() && change.peak_error.is_finite());
                    assert!(
                        change.peak_relative_error.is_finite() && change.reference_peak.is_finite()
                    );
                }
            }
            assert!(quantity.pair(5).is_err());
            assert!(quantity.extrema(usize::MAX).is_err());
        }
    }
    assert_eq!(sampling.next_time(), None);
    assert!(matches!(
        sampling.measure(&family),
        Err(FamilyError::InvalidFamily)
    ));
    assert_eq!(sampling.charged_work(), sampling_plan.bounds().work);
    assert!(matches!(
        sampling.measure(&family),
        Err(FamilyError::Numerical(SolverError::ProviderBudgetExceeded))
    ));
}
