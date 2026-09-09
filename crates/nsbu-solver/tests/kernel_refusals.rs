//! Kernel preflight and nonfinite result refusals for the standalone research interface.
use nsbu_solver::domain::TickClock;
use nsbu_solver::integrators::{
    coefficients::CmCoefficients,
    kernel::{CmWorkspace, RightHandSide},
};
use nsbu_solver::{Complex64, SolverError};

struct Infinite(bool);
impl RightHandSide for Infinite {
    fn evaluate(
        &mut self,
        _state: [&[Complex64]; 3],
        _time: TickClock,
        output: [&mut [Complex64]; 3],
    ) -> Result<(), SolverError> {
        for values in output {
            let value = if self.0 {
                Complex64::new(0.0, f64::INFINITY)
            } else {
                Complex64::new(f64::INFINITY, 0.0)
            };
            values.fill(value);
        }
        Ok(())
    }
}

#[test]
fn kernel_preflight_refuses_invalid_sizes_and_insufficient_storage() {
    assert_eq!(
        CmWorkspace::reservation(0),
        Err(SolverError::InvalidPayload)
    );
    assert_eq!(
        CmWorkspace::reservation(usize::MAX),
        Err(SolverError::SizeOverflow)
    );
    let bytes = CmWorkspace::reservation(2).unwrap();
    assert_eq!(
        bytes,
        15 * 2 * std::mem::size_of::<Complex64>() + std::mem::size_of::<CmWorkspace>()
    );
    assert!(matches!(
        CmWorkspace::new(2, bytes - 1),
        Err(SolverError::ResourceLimit)
    ));
    assert!(CmWorkspace::new(2, bytes).is_ok());
}

#[test]
fn invalid_buffers_intervals_and_nonfinite_results_are_refused() {
    let clock = TickClock::from_rest(-6, 64).unwrap();
    let times = [clock; 3];
    let input = [Complex64::new(0.0, 0.0); 2];
    let coefficients = [CmCoefficients::new(0.0).unwrap(); 2];
    let mut work = CmWorkspace::new(2, 4096).unwrap();
    assert!(Infinite(false).bounds().is_none());
    for (dt, inputs, table, outputs) in [
        (0.0, 2, 2, 2),
        (-1.0, 2, 2, 2),
        (f64::NAN, 2, 2, 2),
        (1.0, 1, 2, 2),
        (1.0, 2, 1, 2),
        (1.0, 2, 2, 1),
    ] {
        let mut out = [input; 3];
        let [a, b, c] = &mut out;
        assert_eq!(
            work.step(
                [&input[..inputs]; 3],
                times,
                dt,
                &coefficients[..table],
                &mut Infinite(false),
                [&mut a[..outputs], b, c]
            ),
            Err(SolverError::InvalidPayload)
        );
    }
    for imaginary in [false, true] {
        let mut out = [input; 3];
        let [a, b, c] = &mut out;
        assert_eq!(
            work.step(
                [&input; 3],
                times,
                1.0,
                &coefficients,
                &mut Infinite(imaginary),
                [a, b, c]
            ),
            Err(SolverError::InvalidSpectrum)
        );
    }
}

struct FiniteEndpoint {
    active_call: usize,
    calls: usize,
    imaginary: bool,
}
impl RightHandSide for FiniteEndpoint {
    fn evaluate(
        &mut self,
        _state: [&[Complex64]; 3],
        _time: TickClock,
        output: [&mut [Complex64]; 3],
    ) -> Result<(), SolverError> {
        self.calls += 1;
        let maximum = if self.calls == self.active_call {
            f64::MAX
        } else {
            0.0
        };
        let value = if self.imaginary {
            Complex64::new(0.0, maximum)
        } else {
            Complex64::new(maximum, 0.0)
        };
        for values in output {
            values.fill(value);
        }
        Ok(())
    }
}

#[test]
fn finite_stage_or_final_arithmetic_overflow_is_refused() {
    let clock = TickClock::from_rest(0, 16).unwrap();
    let input = [Complex64::new(0.0, 0.0)];
    let mut work = CmWorkspace::new(1, 4096).unwrap();
    for (active_call, ticks, imaginary) in [
        (1, 4, false),
        (2, 4, false),
        (3, 4, false),
        (4, 8, false),
        (4, 8, true),
    ] {
        let stages = clock.stages(ticks).unwrap();
        let mut rhs = FiniteEndpoint {
            calls: 0,
            imaginary,
            active_call,
        };
        let mut out = [input; 3];
        let [a, b, c] = &mut out;
        assert_eq!(
            work.step(
                [&input; 3],
                [stages[0], stages[2], stages[4]],
                ticks as f64,
                &[CmCoefficients::new(0.0).unwrap()],
                &mut rhs,
                [a, b, c]
            ),
            Err(SolverError::InvalidSpectrum)
        );
        assert_eq!(rhs.calls, active_call);
    }
}
