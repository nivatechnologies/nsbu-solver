//! Attempt records and concise stdout reporting for the harness-owned schema.
use crate::{artifact, schedule};
use nsbu_solver::{integrators::attempt::AttemptResult, SolverError};

#[derive(Clone, Copy, Default)]
pub struct ObservationTiming {
    pub total: f64,
    pub force: f64,
    pub conservative: f64,
    pub transfer_measure: f64,
}

#[derive(Clone, Copy)]
pub struct AttemptFacts {
    pub index: usize,
    pub clock: u128,
    pub integration_seconds: f64,
    pub ticks: u128,
    pub rhs_calls: usize,
    pub ratios: [f64; 2],
    pub hit_miss: [usize; 2],
}

pub fn committed(identity: &str, timing: Option<ObservationTiming>, facts: AttemptFacts) -> String {
    let observer = timing.map_or_else(|| "null".to_owned(), |value| format!("{:.9}", value.total));
    format!(
        concat!(
            "{{\n  \"schema\": \"p10-avx-scheduled-attempt-v2\",\n",
            "  \"identity\": {},\n  \"attempt\": {},\n  \"attempted_from\": {},\n",
            "  \"attempted_to\": {},\n  \"ticks\": {},\n  \"outcome\": \"committed\",\n",
            "  \"rhs_calls\": {},\n  \"cache_hits\": {},\n  \"cache_misses\": {},\n",
            "  \"integration_seconds\": {:.9},\n  \"observer_seconds\": {},\n",
            "  \"error_ratio_l2\": {:.17e},\n  \"error_ratio_h1\": {:.17e},\n",
            "  \"steady_allocations\": 0\n}}\n"
        ),
        artifact::json_string(identity),
        facts.index,
        facts.clock - facts.ticks,
        facts.clock,
        facts.ticks,
        facts.rhs_calls,
        facts.hit_miss[0],
        facts.hit_miss[1],
        facts.integration_seconds,
        observer,
        facts.ratios[0],
        facts.ratios[1],
    )
}

pub fn rejected(
    identity: &str,
    index: usize,
    from: u128,
    seconds: f64,
    result: &AttemptResult,
) -> String {
    format!(
        concat!(
            "{{\n  \"schema\": \"p10-avx-scheduled-attempt-v2\",\n",
            "  \"identity\": {},\n  \"attempt\": {},\n  \"attempted_from\": {},\n",
            "  \"attempted_to\": {},\n  \"ticks\": {},\n  \"outcome\": \"rejected\",\n",
            "  \"integration_seconds\": {:.9},\n  \"error_ratio_l2\": {:.17e},\n",
            "  \"error_ratio_h1\": {:.17e}\n}}\n"
        ),
        artifact::json_string(identity),
        index,
        from,
        from + result.ticks,
        result.ticks,
        seconds,
        result.indicators.ratios[0],
        result.indicators.ratios[1],
    )
}

pub fn numerical_error(
    identity: &str,
    index: usize,
    from: u128,
    seconds: f64,
    error: &SolverError,
) -> String {
    format!(
        concat!(
            "{{\n  \"schema\": \"p10-avx-scheduled-attempt-v2\",\n",
            "  \"identity\": {},\n  \"attempt\": {},\n  \"attempted_from\": {},\n",
            "  \"attempted_to\": {},\n  \"ticks\": {},\n  \"outcome\": \"numerical_error\",\n",
            "  \"integration_seconds\": {:.9},\n  \"error\": {}\n}}\n"
        ),
        artifact::json_string(identity),
        index,
        from,
        from + schedule::STEP,
        schedule::STEP,
        seconds,
        artifact::json_string(&format!("{error:?}")),
    )
}

pub fn report(
    facts: AttemptFacts,
    timing: Option<ObservationTiming>,
    publication: crate::artifact::PublicationKind,
    hash: Option<&str>,
) {
    println!(
        "attempt={} clock={} integration_seconds={:.9} cache_hit_miss={:?} observer_seconds={:?} ratios={:?} publication={publication:?} state_sha256={hash:?} steady_allocations=0",
        facts.index,
        facts.clock,
        facts.integration_seconds,
        facts.hit_miss,
        timing.map(|value| value.total),
        facts.ratios,
    );
}
