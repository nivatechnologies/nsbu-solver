//! Transparent harness-only attribution of wall time spent inside RHS evaluations.
use nsbu_solver::{
    domain::TickClock,
    integrators::{
        kernel::{RhsBounds, RightHandSide},
        method::Method,
    },
    Complex64, SolverError,
};
use std::time::Instant;

pub const IDENTITY: &str =
    "harness-timed-rhs-v1;clock=std-time-Instant;scope=evaluate-inclusive;overhead=included";

#[derive(Clone, Copy, Debug, Default)]
pub struct Measurement {
    pub calls: usize,
    pub seconds: f64,
}

pub struct TimedRhs<R> {
    inner: R,
    measurement: Measurement,
}

impl<R> TimedRhs<R> {
    pub fn new(inner: R) -> Self {
        Self {
            inner,
            measurement: Measurement::default(),
        }
    }

    pub fn inner(&self) -> &R {
        &self.inner
    }

    pub fn reset_measurement(&mut self) {
        self.measurement = Measurement::default();
    }

    pub fn measurement(&self) -> Measurement {
        self.measurement
    }

    pub const fn reservation_overhead() -> usize {
        std::mem::size_of::<Self>() - std::mem::size_of::<R>()
    }
}

impl<R: RightHandSide> RightHandSide for TimedRhs<R> {
    fn bounds(&self) -> Option<RhsBounds> {
        self.inner.bounds()
    }

    fn begin_attempt(&mut self, clock: TickClock, ticks: u128) -> Result<(), SolverError> {
        self.reset_measurement();
        self.inner.begin_attempt(clock, ticks)
    }

    fn begin_attempt_for_method(
        &mut self,
        clock: TickClock,
        ticks: u128,
        method: Method,
    ) -> Result<(), SolverError> {
        self.reset_measurement();
        self.inner.begin_attempt_for_method(clock, ticks, method)
    }

    fn evaluate(
        &mut self,
        state: [&[Complex64]; 3],
        time: TickClock,
        output: [&mut [Complex64]; 3],
    ) -> Result<(), SolverError> {
        let started = Instant::now();
        let result = self.inner.evaluate(state, time, output);
        self.measurement.calls += 1;
        self.measurement.seconds += started.elapsed().as_secs_f64();
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Default)]
    struct Stub {
        began: usize,
        evaluated: usize,
    }

    impl RightHandSide for Stub {
        fn bounds(&self) -> Option<RhsBounds> {
            Some(RhsBounds {
                storage_bytes: 17,
                work_units: 2,
                scalar_transforms: 3,
            })
        }

        fn begin_attempt(&mut self, _clock: TickClock, _ticks: u128) -> Result<(), SolverError> {
            self.began += 1;
            Ok(())
        }

        fn evaluate(
            &mut self,
            state: [&[Complex64]; 3],
            _time: TickClock,
            output: [&mut [Complex64]; 3],
        ) -> Result<(), SolverError> {
            self.evaluated += 1;
            for axis in 0..3 {
                output[axis].copy_from_slice(state[axis]);
            }
            Ok(())
        }
    }

    #[test]
    fn delegates_and_accounts_each_evaluation() {
        let clock = TickClock::from_rest(-20, 8192).unwrap();
        let mut rhs = TimedRhs::new(Stub::default());
        let input = [[Complex64::new(1.0, -1.0); 1]; 3];
        let mut output = [[Complex64::new(0.0, 0.0); 1]; 3];
        rhs.begin_attempt_for_method(clock, 32, Method::CoxMatthews)
            .unwrap();
        let [out0, out1, out2] = &mut output;
        rhs.evaluate([&input[0], &input[1], &input[2]], clock, [out0, out1, out2])
            .unwrap();
        assert_eq!(rhs.bounds().unwrap().storage_bytes, 17);
        assert_eq!(rhs.inner().began, 1);
        assert_eq!(rhs.inner().evaluated, 1);
        assert_eq!(rhs.measurement().calls, 1);
        assert!(rhs.measurement().seconds.is_finite());
        assert_eq!(output, input);
    }
}
