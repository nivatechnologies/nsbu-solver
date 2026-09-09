//! Exact integer fixtures, including a misleading binary64 sum counterexample.
use nsbu_solver::domain::{Epoch, TickClock};
use nsbu_solver::SolverError;

#[test]
fn exact_quarter_stages_use_the_committed_counts() {
    let clock = TickClock::restore(-20, 8192, 4000, 4192).unwrap();
    let stages = clock.stages(128).unwrap();
    for (index, stage) in stages.into_iter().enumerate() {
        assert_eq!(stage.exponent(), -20);
        assert_eq!(stage.target(), 8192);
        assert_eq!(stage.elapsed(), 4000 + 32 * index as u128);
        assert_eq!(stage.remaining(), 4192 - 32 * index as u128);
    }
    assert_eq!(clock.elapsed(), 4000);
    assert_eq!(clock.remaining(), 4192);
    assert_eq!(clock.stages(4).unwrap()[4].remaining(), 4188);
}

#[test]
fn clock_refuses_invalid_states_and_intervals() {
    assert_eq!(TickClock::from_rest(0, 0), Err(SolverError::InvalidClock));
    assert_eq!(
        TickClock::restore(0, 8, 8, 0),
        Err(SolverError::InvalidClock)
    );
    assert_eq!(
        TickClock::restore(0, 8, 2, 5),
        Err(SolverError::InvalidClock)
    );
    assert_eq!(
        TickClock::restore(0, 8, 2, 7),
        Err(SolverError::InvalidClock)
    );
    assert_eq!(
        TickClock::restore(0, 8, u128::MAX, 1),
        Err(SolverError::ClockCapacityExceeded)
    );
    let clock = TickClock::from_rest(-7, 8).unwrap();
    assert_eq!(clock.elapsed(), 0);
    assert_eq!(clock.remaining(), 8);
    for ticks in [0, 1, 2, 3, 5, u128::MAX] {
        assert_eq!(clock.stages(ticks), Err(SolverError::InvalidStep));
    }
    for ticks in [8, 12, u128::MAX - 3] {
        assert_eq!(clock.stages(ticks), Err(SolverError::ClockCapacityExceeded));
    }
    assert_eq!(clock.stages(4).unwrap()[4].elapsed(), 4);
}

#[test]
fn finite_clock_capacity_and_extreme_quantum_are_explicit() {
    let clock = TickClock::restore(i32::MIN, u128::MAX, u128::MAX - 5, 5).unwrap();
    let end = clock.stages(4).unwrap()[4];
    assert_eq!(end.elapsed(), u128::MAX - 1);
    assert_eq!(end.remaining(), 1);
    assert_eq!(end.exponent(), i32::MIN);
    assert_eq!(end.stages(4), Err(SolverError::ClockCapacityExceeded));
    assert_eq!(
        TickClock::from_rest(i32::MAX, 1).unwrap().exponent(),
        i32::MAX
    );
    assert_eq!(Epoch(0).next(), Ok(Epoch(1)));
    assert_eq!(Epoch(u128::MAX - 1).next(), Ok(Epoch(u128::MAX)));
    assert_eq!(Epoch(u128::MAX).next(), Err(SolverError::EpochExhausted));
}

#[test]
fn floating_sum_can_pass_while_elapsed_time_does_not_advance() {
    let elapsed = 0.5_f64;
    let remaining = 0.5_f64;
    let half_ulp = 2.0_f64.powi(-54);
    let next_elapsed = elapsed + half_ulp;
    let next_remaining = remaining - half_ulp;
    assert_eq!(next_elapsed, elapsed);
    assert_eq!(next_elapsed + next_remaining, 1.0);
    // Exact conversion of each individual dyadic reveals the lost tick.
    assert_eq!((next_elapsed * 2.0_f64.powi(54)) as u128, 1 << 53);
    assert_eq!((next_remaining * 2.0_f64.powi(54)) as u128, (1 << 53) - 1);
    let exact = TickClock::restore(-56, 1 << 56, 1 << 55, 1 << 55).unwrap();
    let end = exact.stages(4).unwrap()[4];
    assert_eq!(end.elapsed(), (1 << 55) + 4);
    assert_eq!(end.remaining(), (1 << 55) - 4);
    assert_eq!(end.elapsed() + end.remaining(), end.target());
}
