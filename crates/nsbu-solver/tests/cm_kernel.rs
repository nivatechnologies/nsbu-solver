//! Single-step identities, actual forcing clocks and failure propagation.
mod support;
use nsbu_solver::domain::TickClock;
use nsbu_solver::integrators::{
    coefficients::CmCoefficients,
    kernel::{CmWorkspace, RightHandSide},
};
use nsbu_solver::{Complex64, SolverError};
use support::CallLog;

struct Probe {
    log: CallLog,
    source: f64,
    linear: f64,
}

impl RightHandSide for Probe {
    fn evaluate(
        &mut self,
        state: [&[Complex64]; 3],
        time: TickClock,
        output: [&mut [Complex64]; 3],
    ) -> Result<(), SolverError> {
        self.log.record(time)?;
        for (axis, values) in output.into_iter().enumerate() {
            for (i, v) in values.iter_mut().enumerate() {
                *v = self.source + self.linear * state[axis][i];
            }
        }
        Ok(())
    }
}

#[test]
fn diffusion_constant_source_rk4_and_actual_stage_clocks() {
    let clock = TickClock::from_rest(-6, 64).unwrap();
    let stages = clock.stages(8).unwrap();
    let times = [stages[0], stages[2], stages[4]];
    let dt = 0.125;
    let initial = [Complex64::new(0.7, -0.2)];
    let mut work = CmWorkspace::new(1, 4096).unwrap();
    for (z, source, linear) in [(-1.0, 0.0, 0.0), (-1.0, 2.0, 0.0), (0.0, 0.0, 3.0)] {
        let coefficients = [CmCoefficients::new(z).unwrap()];
        let mut probe = Probe {
            log: CallLog {
                ticks: Vec::with_capacity(4),
                fail_at: 0,
            },
            source,
            linear,
        };
        let mut output = [[Complex64::new(0.0, 0.0)]; 3];
        let [a, b, c] = &mut output;
        work.step(
            [&initial; 3],
            times,
            dt,
            &coefficients,
            &mut probe,
            [a, b, c],
        )
        .unwrap();
        assert_eq!(probe.log.ticks, [0, 4, 4, 8]);
        let expected = if z == 0.0 {
            let w = dt * linear;
            initial[0] * (1.0 + w + w * w / 2.0 + w.powi(3) / 6.0 + w.powi(4) / 24.0)
        } else {
            initial[0] * z.exp() + source * dt * z.exp_m1() / z
        };
        for values in output {
            assert!((values[0] - expected).norm_sqr() < 1e-28);
        }
    }
}

#[test]
fn every_rhs_failure_propagates_without_changing_input() {
    let stages = TickClock::from_rest(-6, 64).unwrap().stages(8).unwrap();
    let initial = [Complex64::new(0.3, 0.0)];
    let mut work = CmWorkspace::new(1, 4096).unwrap();
    for fail_at in 1..=4 {
        let mut probe = Probe {
            log: CallLog {
                ticks: Vec::with_capacity(4),
                fail_at,
            },
            source: 0.0,
            linear: 1.0,
        };
        let mut output = [[Complex64::new(0.0, 0.0)]; 3];
        let [a, b, c] = &mut output;
        assert_eq!(
            work.step(
                [&initial; 3],
                [stages[0], stages[2], stages[4]],
                0.125,
                &[CmCoefficients::new(0.0).unwrap()],
                &mut probe,
                [a, b, c]
            ),
            Err(SolverError::ResourceLimit)
        );
        assert_eq!(probe.log.ticks.len(), fail_at);
        assert_eq!(initial, [Complex64::new(0.3, 0.0)]);
    }
}
