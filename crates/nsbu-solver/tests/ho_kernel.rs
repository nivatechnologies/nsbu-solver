//! Nonautonomous stiff scalar trajectories and the actual nonmonotone HO request sequence.
mod support;
use nsbu_solver::{
    domain::TickClock,
    integrators::{ho_coefficients::HoCoefficients, ho_kernel::HoWorkspace, kernel::RightHandSide},
    Complex64, SolverError,
};
use support::CallLog;

struct Source {
    decay: f64,
    frequency: f64,
    log: CallLog,
}
impl RightHandSide for Source {
    fn evaluate(
        &mut self,
        state: [&[Complex64]; 3],
        clock: TickClock,
        output: [&mut [Complex64]; 3],
    ) -> Result<(), SolverError> {
        self.log.record(clock)?;
        let time = clock.elapsed() as f64 * 2.0_f64.powi(clock.exponent());
        let phase = self.frequency * time;
        let force = self.frequency * phase.cos() + (self.decay + 2.0) * phase.sin();
        for (axis, values) in output.into_iter().enumerate() {
            values[0] = force - 2.0 * state[axis][0];
        }
        Ok(())
    }
}

fn run(decay: f64, steps: usize, frequency: f64) -> f64 {
    let mut clock = TickClock::from_rest(-20, 1 << 20).unwrap();
    let ticks = (1 << 16) / steps as u128;
    let dt = ticks as f64 / (1 << 20) as f64;
    let table = [HoCoefficients::new(-decay * dt).unwrap()];
    let mut workspace = HoWorkspace::new(1, 4096).unwrap();
    let mut source = Source {
        decay,
        frequency,
        log: CallLog {
            ticks: Vec::with_capacity(5 * steps),
            fail_at: 0,
        },
    };
    let mut state = [[Complex64::new(0.0, 0.0)]; 3];
    let mut output = state;
    for _ in 0..steps {
        let stages = clock.stages(ticks).unwrap();
        let [a, b, c] = &mut output;
        workspace
            .step(
                [&state[0], &state[1], &state[2]],
                [stages[0], stages[2], stages[4]],
                dt,
                &table,
                &mut source,
                [a, b, c],
            )
            .unwrap();
        state = output;
        clock = stages[4];
    }
    assert_eq!(source.log.ticks.len(), 5 * steps);
    assert_eq!(clock.elapsed(), 1 << 16);
    state[0][0].re
}

fn trajectory_error(decay: f64, steps: usize, frequency: f64) -> f64 {
    (run(decay, steps, frequency) - (frequency / 16.0).sin()).abs()
}

#[test]
fn independently_evolved_stiff_nonautonomous_refinement() {
    for (decay, frequency, base_steps) in [
        (0.0, 13.0, 8),
        (20.0, 13.0, 8),
        (1000.0, 13.0, 32),
        (100000.0, 1300.0, 2048),
    ] {
        let mut previous = None;
        for multiple in [1, 2, 4, 8] {
            let steps = base_steps * multiple;
            let error = trajectory_error(decay, steps, frequency);
            println!(
                "HO refined decay={decay} frequency={frequency} steps={steps} error={error:e}"
            );
            if let Some(coarse) = previous {
                let order = f64::log2(coarse / error);
                println!("HO refined order={order}");
                assert!(order > 3.5);
                assert!(order < 4.5);
            }
            previous = Some(error);
        }
    }
}

#[test]
fn coarse_stiff_steps_retain_the_observed_order_reduction() {
    let mut previous = None;
    for steps in [8, 16, 32, 64] {
        let error = trajectory_error(100000.0, steps, 13.0);
        if let Some(coarse) = previous {
            let order = f64::log2(coarse / error);
            assert!(order > 1.9);
            assert!(order < 2.2);
        }
        previous = Some(error);
    }
}

#[test]
fn fifth_request_returns_to_half_time_and_all_failures_propagate() {
    let clock = TickClock::from_rest(-6, 64).unwrap();
    let stages = clock.stages(8).unwrap();
    let input = [Complex64::new(0.0, 0.0)];
    let mut workspace = HoWorkspace::new(1, 4096).unwrap();
    for fail_at in 0..=5 {
        let mut source = Source {
            decay: 1.0,
            frequency: 13.0,
            log: CallLog {
                ticks: Vec::with_capacity(5),
                fail_at,
            },
        };
        let mut output = [input; 3];
        let [a, b, c] = &mut output;
        let result = short_step(&mut workspace, &input, stages, &mut source, [a, b, c]);
        if fail_at == 0 {
            result.unwrap();
            assert_eq!(source.log.ticks, [0, 4, 4, 8, 4]);
        } else {
            assert_eq!(result, Err(SolverError::ResourceLimit));
            assert_eq!(source.log.ticks.len(), fail_at);
        }
        assert_eq!(input, [Complex64::new(0.0, 0.0)]);
    }
}

struct IncreasingOnly(Source);
impl RightHandSide for IncreasingOnly {
    fn evaluate(
        &mut self,
        state: [&[Complex64]; 3],
        clock: TickClock,
        output: [&mut [Complex64]; 3],
    ) -> Result<(), SolverError> {
        if self
            .0
            .log
            .ticks
            .last()
            .is_some_and(|previous| clock.elapsed() < *previous)
        {
            return Err(SolverError::InvalidClock);
        }
        self.0.evaluate(state, clock, output)
    }
}

#[test]
fn a_provider_that_assumes_increasing_times_fails_the_ho_purity_control() {
    let stages = TickClock::from_rest(-6, 64).unwrap().stages(8).unwrap();
    let mut source = IncreasingOnly(Source {
        decay: 1.0,
        frequency: 13.0,
        log: CallLog {
            ticks: Vec::with_capacity(5),
            fail_at: 0,
        },
    });
    let mut workspace = HoWorkspace::new(1, 4096).unwrap();
    let zero = [Complex64::new(0.0, 0.0)];
    let mut output = [zero; 3];
    let [a, b, c] = &mut output;
    let result = short_step(&mut workspace, &zero, stages, &mut source, [a, b, c]);
    assert_eq!(result, Err(SolverError::InvalidClock));
    assert_eq!(source.0.log.ticks, [0, 4, 4, 8]);
}

fn short_step(
    workspace: &mut HoWorkspace,
    input: &[Complex64; 1],
    stages: [TickClock; 5],
    source: &mut dyn RightHandSide,
    output: [&mut [Complex64]; 3],
) -> Result<(), SolverError> {
    workspace.step(
        [input; 3],
        [stages[0], stages[2], stages[4]],
        0.125,
        &[HoCoefficients::new(-0.125).unwrap()],
        source,
        output,
    )
}

#[test]
fn stiff_final_states_match_independent_high_precision_evolution() {
    let mut maximum = 0.0_f64;
    for line in include_str!("fixtures/ho-stiff-trajectories.tsv").lines() {
        let columns: Vec<_> = line.split('\t').collect();
        let decay = columns[0].parse().unwrap();
        let frequency = columns[1].parse().unwrap();
        let steps = columns[2].parse().unwrap();
        let expected: f64 = columns[3].parse().unwrap();
        let actual = run(decay, steps, frequency);
        println!(
            "HO scalar actual decay={decay} frequency={frequency} steps={steps} value={actual:e}"
        );
        let difference = (actual - expected).abs();
        maximum = maximum.max(difference);
        assert!(difference < 4e-13);
    }
    println!("HO scalar trajectories maximum absolute difference versus independent 120-digit states={maximum:e}");
}
