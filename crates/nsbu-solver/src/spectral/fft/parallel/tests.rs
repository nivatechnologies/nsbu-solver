//! Failure controls for the private executor boundary.
use super::*;

fn executor() -> ParallelFftExecutor {
    let layout = Layout::new([6; 3]).unwrap();
    let backend = FftBackend::RustFft6_4_1AvxFma;
    let cap = ParallelFftExecutor::additional_reservation(layout, backend, 2).unwrap();
    ParallelFftExecutor::new(layout, backend, 2, cap).unwrap()
}

#[test]
fn panic_terminates_executor_and_refuses_later_work() {
    if FftBackend::RustFft6_4_1AvxFma.ensure_available().is_err() {
        return;
    }
    let executor = executor();
    assert_eq!(
        executor.execute(|| panic!("worker panic control")),
        Err(SolverError::ArithmeticResolutionLimited)
    );
    assert!(executor.is_terminated());
    assert_eq!(
        executor.execute(|| panic!("must never run")),
        Err(SolverError::ArithmeticResolutionLimited)
    );
    executor.line(|_| panic!("terminated executor must refuse line"));
}

#[test]
fn poisoned_scratch_is_a_permanent_failure() {
    if FftBackend::RustFft6_4_1AvxFma.ensure_available().is_err() {
        return;
    }
    let executor = executor();
    for line in &executor.lines {
        assert!(catch_unwind(AssertUnwindSafe(|| {
            let _guard = line.lock().unwrap();
            panic!("scratch poison control");
        }))
        .is_err());
    }
    assert_eq!(
        executor.execute(|| executor.line(|_| panic!("poisoned line used"))),
        Err(SolverError::ArithmeticResolutionLimited)
    );
    assert!(executor.is_terminated());
}

#[test]
fn invalid_inputs_refuse_before_transform_without_terminating_owner() {
    if FftBackend::RustFft6_4_1AvxFma.ensure_available().is_err() {
        return;
    }
    let executor = executor();
    let identity = executor.identity();
    let cap = FftPlan::reservation_with_backend(identity.layout, identity.backend).unwrap();
    let (plan, mut work) =
        FftPlan::new_with_backend(identity.layout, identity.backend, cap).unwrap();
    let mut input = vec![0.0; identity.layout.real_len()];
    let mut output = vec![Complex64::new(0.0, 0.0); identity.layout.half_len()];
    assert_eq!(
        executor.forward(&plan, &input[1..], &mut output, &mut work),
        Err(SolverError::InvalidPayload)
    );
    assert_eq!(
        executor.forward(&plan, &input, &mut output[1..], &mut work),
        Err(SolverError::InvalidPayload)
    );
    input[0] = f64::NAN;
    assert!(executor
        .forward(&plan, &input, &mut output, &mut work)
        .is_err());
    assert!(output.iter().all(|v| *v == Complex64::new(0.0, 0.0)));
    output[0].im = 1.0;
    assert!(executor
        .inverse(&plan, &output, &mut input, &mut work)
        .is_err());
    assert!(!executor.is_terminated());
    work.layout = Layout::new([96, 6, 6]).unwrap();
    assert_eq!(
        executor.forward(&plan, &input, &mut output, &mut work),
        Err(SolverError::InvalidPayload)
    );
}
