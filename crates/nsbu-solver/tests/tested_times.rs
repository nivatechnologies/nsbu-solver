//! Common exact samples must survive refinement, including beyond binary64 integer resolution.
use nsbu_solver::{
    domain::TickClock,
    verification::{times::TestedTimes, VerificationError},
};

fn clocks(ticks: &[u128]) -> Vec<TickClock> {
    ticks
        .iter()
        .map(|&elapsed| TickClock::restore(-80, 1 << 70, elapsed, (1 << 70) - elapsed).unwrap())
        .collect()
}

#[test]
fn strict_nested_sampling_preserves_exact_times_and_the_window_endpoint() {
    let a = clocks(&[0, 1 << 60, (1 << 60) + 4]);
    let b = clocks(&[0, 1 << 59, 1 << 60, (1 << 60) + 1, (1 << 60) + 4]);
    let coarse = TestedTimes::new(&a, 3).unwrap();
    let fine = TestedTimes::new(&b, 5).unwrap();
    assert_eq!(fine.as_slice(), b);
    assert!(fine.refines(coarse));
    assert!(!coarse.refines(fine));
    assert!(!fine.refines(fine));
    for ticks in [
        vec![0, 1, (1 << 60) - 1, (1 << 60) + 1, (1 << 60) + 4],
        vec![0, 1 << 60, (1 << 60) + 1, (1 << 60) + 4, (1 << 60) + 5],
    ] {
        let values = clocks(&ticks);
        assert!(!TestedTimes::new(&values, 5).unwrap().refines(coarse));
    }
    let different = b
        .iter()
        .map(|c| TickClock::restore(-81, c.target(), c.elapsed(), c.remaining()).unwrap())
        .collect::<Vec<_>>();
    assert!(!TestedTimes::new(&different, 5).unwrap().refines(coarse));
}

#[test]
fn empty_unsorted_duplicate_and_mixed_clock_manifests_are_refused() {
    for ticks in [
        vec![],
        vec![0],
        vec![1, 2],
        vec![0, 0],
        vec![0, 2, 1],
        vec![0, 1, 1],
    ] {
        let values = clocks(&ticks);
        assert_eq!(
            TestedTimes::new(&values, 3).unwrap_err(),
            VerificationError::InvalidTimes
        );
    }
    let a = clocks(&[0, 1]);
    assert_eq!(
        TestedTimes::new(&a, 1).unwrap_err(),
        VerificationError::CapacityExceeded
    );
    for changed in [
        TickClock::restore(-81, 1 << 70, 1, (1 << 70) - 1).unwrap(),
        TickClock::restore(-80, 1 << 71, 1, (1 << 71) - 1).unwrap(),
    ] {
        let mixed = [a[0], changed];
        assert_eq!(
            TestedTimes::new(&mixed, 2).unwrap_err(),
            VerificationError::InvalidTimes
        );
    }
}
