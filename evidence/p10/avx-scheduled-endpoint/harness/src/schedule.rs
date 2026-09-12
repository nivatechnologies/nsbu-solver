//! Immutable nested Simpson clock sets for the first legal endpoint.
use nsbu_solver::SolverError;

pub const ENDPOINT: u128 = 4096;
#[cfg(not(feature = "n384-h64"))]
pub const STEP: u128 = 32;
#[cfg(feature = "n384-h64")]
pub const STEP: u128 = 64;
#[cfg(not(feature = "n384-h64"))]
pub const MAXIMUM_ATTEMPTS: usize = 128;
#[cfg(feature = "n384-h64")]
pub const MAXIMUM_ATTEMPTS: usize = 64;
pub const FINE: [u128; 9] = [0, 512, 1024, 1536, 2048, 2560, 3072, 3584, 4096];
pub const MIDDLE: [u128; 5] = [0, 1024, 2048, 3072, 4096];
pub const COARSE: [u128; 3] = [0, 2048, 4096];

pub fn positive_node(clock: u128) -> bool {
    clock > 0 && FINE.contains(&clock)
}

pub fn validate() -> Result<(), SolverError> {
    if STEP
        .checked_mul(MAXIMUM_ATTEMPTS as u128)
        .ok_or(SolverError::SizeOverflow)?
        != ENDPOINT
        || FINE[0] != 0
        || FINE[FINE.len() - 1] != ENDPOINT
        || !nested(&MIDDLE, &FINE)
        || !nested(&COARSE, &MIDDLE)
        || !simpson(&FINE)
        || !simpson(&MIDDLE)
        || !simpson(&COARSE)
    {
        return Err(SolverError::InvalidClock);
    }
    Ok(())
}

fn nested(coarse: &[u128], fine: &[u128]) -> bool {
    coarse.iter().all(|clock| fine.contains(clock))
}

fn simpson(nodes: &[u128]) -> bool {
    if nodes.len() < 3 || !(nodes.len() - 1).is_multiple_of(2) {
        return false;
    }
    let spacing = nodes[1] - nodes[0];
    spacing > 0 && nodes.windows(2).all(|pair| pair[1] - pair[0] == spacing)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schedule_is_exact_nested_and_has_eight_positive_observations() {
        validate().unwrap();
        assert_eq!(
            FINE.iter()
                .copied()
                .filter(|&clock| positive_node(clock))
                .count(),
            8
        );
        assert_eq!(ENDPOINT / STEP, MAXIMUM_ATTEMPTS as u128);
    }

    #[test]
    fn unscheduled_and_rest_clocks_do_not_trigger_positive_observation() {
        for clock in [0, STEP, 511, 513, 4095] {
            assert!(!positive_node(clock));
        }
    }
}
