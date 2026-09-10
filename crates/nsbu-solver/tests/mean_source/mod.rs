//! Shared spatially uniform cosine source for recorded and basic trajectory fixtures.
use super::source_contract;
use nsbu_solver::Complex64;
pub fn source(fail_at: usize) -> source_contract::Source {
    source_contract::Source::new(
        fail_at,
        [[Complex64::new(1.0, 0.0), Complex64::new(0.0, 0.0)]; 3],
    )
}
