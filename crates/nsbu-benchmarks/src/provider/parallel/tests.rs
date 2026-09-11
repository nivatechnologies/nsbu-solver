//! Worker failures drain the whole attempt without publishing earlier successful planes.
use super::*;
fn clock(t: u128) -> TickClock {
    TickClock::restore(-10, 8, t, 8 - t).unwrap()
}
#[test]
fn each_failed_worker_preserves_previous_coefficients_and_terminates_the_whole_provider() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let samples = Layout::new([6, 8, 12]).unwrap();
    for failed in 0..3 {
        let limits = ParallelV2Force::preflight(domain, samples, 3).unwrap();
        let mut provider = ParallelV2Force::new(domain, samples, 3, limits.storage_bytes).unwrap();
        let mut output: [Vec<Complex64>; 3] =
            std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); domain.layout().half_len()]);
        provider
            .evaluate(clock(1), limits, output.each_mut().map(Vec::as_mut_slice))
            .unwrap();
        let prior = output.clone();
        let roots = provider.last_root_iterations();
        assert!(roots > 0);
        provider.pool.fail_next(failed);
        assert_eq!(
            provider
                .evaluate(clock(4), limits, output.each_mut().map(Vec::as_mut_slice))
                .unwrap_err(),
            SolverError::ArithmeticResolutionLimited
        );
        assert_eq!(output, prior);
        assert_eq!(provider.last_root_iterations(), roots);
        assert!(provider.is_terminated());
        assert!(provider.pool.all_collected());
        assert_eq!(
            provider
                .evaluate(clock(1), limits, output.each_mut().map(Vec::as_mut_slice))
                .unwrap_err(),
            SolverError::ProviderBudgetExceeded
        );
        assert_eq!(output, prior);
    }
}

#[test]
fn a_panicked_worker_wakes_the_controller_and_cannot_publish_coefficients() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let samples = Layout::new([4; 3]).unwrap();
    let limits = ParallelV2Force::preflight(domain, samples, 2).unwrap();
    let mut provider = ParallelV2Force::new(domain, samples, 2, limits.storage_bytes).unwrap();
    let mut output: [Vec<Complex64>; 3] =
        std::array::from_fn(|_| vec![Complex64::new(11.0, -7.0); domain.layout().half_len()]);
    let prior = output.clone();
    provider.pool.panic_next(1);
    assert_eq!(
        provider
            .evaluate(clock(1), limits, output.each_mut().map(Vec::as_mut_slice))
            .unwrap_err(),
        SolverError::ArithmeticResolutionLimited
    );
    assert!(provider.is_terminated());
    assert!(provider.pool.all_collected());
    assert_eq!(output, prior);
}
