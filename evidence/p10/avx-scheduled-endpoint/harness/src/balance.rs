//! Fixed-node balance storage, lookup, quadrature, and terminal serialization.
use crate::{artifact, schedule};
use nsbu_solver::{
    diagnostics::{balances::BalanceSample, history::BalanceHistory, quadrature::BalanceIntegral},
    domain::TickClock,
    SolverError,
};

#[derive(Clone, Copy)]
pub struct TimedBalance {
    pub clock: TickClock,
    pub sample: BalanceSample,
}

pub fn storage(state_clock: TickClock) -> Result<Vec<TimedBalance>, SolverError> {
    let mut balances = Vec::new();
    balances
        .try_reserve_exact(schedule::FINE.len())
        .map_err(|_| SolverError::AllocationFailed)?;
    balances.push(TimedBalance {
        clock: state_clock,
        sample: BalanceSample::REST,
    });
    Ok(balances)
}

pub fn quadrature(samples: &[TimedBalance]) -> Result<[BalanceIntegral; 3], SolverError> {
    Ok([
        history(samples, &schedule::COARSE)?,
        history(samples, &schedule::MIDDLE)?,
        history(samples, &schedule::FINE)?,
    ])
}

fn history(samples: &[TimedBalance], clocks: &[u128]) -> Result<BalanceIntegral, SolverError> {
    let initial = find_sample(samples, clocks[0])?;
    let history = BalanceHistory::new(initial.clock, initial.sample, clocks.len())?;
    extend_history(samples, &clocks[1..], history)?.integral()
}

fn extend_history(
    samples: &[TimedBalance],
    clocks: &[u128],
    history: BalanceHistory,
) -> Result<BalanceHistory, SolverError> {
    clocks.iter().try_fold(history, |history, clock| {
        let sample = find_sample(samples, *clock)?;
        history.with_sample(sample.clock, sample.sample)
    })
}

fn find_sample(samples: &[TimedBalance], clock: u128) -> Result<TimedBalance, SolverError> {
    samples
        .iter()
        .copied()
        .find(|sample| sample.clock.elapsed() == clock)
        .ok_or(SolverError::InvalidClock)
}

pub fn terminal_json(
    identity: &str,
    clock: TickClock,
    samples: usize,
    integrals: [BalanceIntegral; 3],
) -> String {
    format!(
        concat!(
            "{{\n  \"status\": \"endpoint_complete_qualification_pending\",\n",
            "  \"identity\": {},\n  \"clock\": {},\n  \"samples\": {},\n",
            "  \"quadrature_sufficiency\": \"not_assessed\",\n",
            "  \"coarse\": {},\n  \"middle\": {},\n  \"fine\": {}\n}}\n"
        ),
        artifact::json_string(identity),
        clock.elapsed(),
        samples,
        artifact::json_string(&format!("{:?}", integrals[0])),
        artifact::json_string(&format!("{:?}", integrals[1])),
        artifact::json_string(&format!("{:?}", integrals[2])),
    )
}
