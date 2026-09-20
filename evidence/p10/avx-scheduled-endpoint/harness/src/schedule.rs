//! Immutable nested Simpson clock sets for the first legal endpoint.
use nsbu_solver::SolverError;

pub const ENDPOINT: u128 = 4096;
#[cfg(all(
    not(feature = "n384-h64"),
    not(feature = "n384-piecewise-common"),
    not(feature = "n512-m512-temporal-h32"),
    not(feature = "n512-m512-temporal-h16")
))]
pub const STEP: u128 = 32;
#[cfg(feature = "n384-h64")]
pub const STEP: u128 = 64;
#[cfg(all(
    not(feature = "n384-h64"),
    not(feature = "n384-piecewise-common"),
    not(feature = "n512-m512-temporal-h32"),
    not(feature = "n512-m512-temporal-h16")
))]
pub const MAXIMUM_ATTEMPTS: usize = 128;
#[cfg(feature = "n384-h64")]
pub const MAXIMUM_ATTEMPTS: usize = 64;
#[cfg(feature = "n384-piecewise-common")]
pub const MAXIMUM_ATTEMPTS: usize = 48;
#[cfg(feature = "n512-m512-temporal-h32")]
pub const MAXIMUM_ATTEMPTS: usize = 96;
#[cfg(feature = "n512-m512-temporal-h16")]
pub const MAXIMUM_ATTEMPTS: usize = 192;
#[cfg(all(
    not(feature = "n384-h64"),
    not(feature = "n384-piecewise-common"),
    not(feature = "n512-m512-temporal-h32"),
    not(feature = "n512-m512-temporal-h16")
))]
pub const IDENTITY: &str = "constant-h32";
#[cfg(feature = "n384-h64")]
pub const IDENTITY: &str = "constant-h64";
#[cfg(feature = "n384-piecewise-common")]
pub const IDENTITY: &str = "h64-clocks0-through2048-then-h128-through4096";
#[cfg(feature = "n512-m512-temporal-h32")]
pub const IDENTITY: &str = "h32-clocks0-through2048-then-h64-through4096";
#[cfg(feature = "n512-m512-temporal-h16")]
pub const IDENTITY: &str = "h16-clocks0-through2048-then-h32-through4096";
pub const FINE: [u128; 9] = [0, 512, 1024, 1536, 2048, 2560, 3072, 3584, 4096];
pub const MIDDLE: [u128; 5] = [0, 1024, 2048, 3072, 4096];
pub const COARSE: [u128; 3] = [0, 2048, 4096];

pub fn positive_node(clock: u128) -> bool {
    clock > 0 && FINE.contains(&clock)
}

fn accepted_clock(clock: u128) -> bool {
    if clock == ENDPOINT {
        return true;
    }
    step(clock).is_ok()
}

#[cfg(all(
    not(feature = "n384-piecewise-common"),
    not(feature = "n512-m512-temporal-h32"),
    not(feature = "n512-m512-temporal-h16")
))]
pub fn step(clock: u128) -> Result<u128, SolverError> {
    if clock < ENDPOINT && clock.is_multiple_of(STEP) {
        Ok(STEP)
    } else {
        Err(SolverError::InvalidClock)
    }
}

#[cfg(feature = "n384-piecewise-common")]
pub fn step(clock: u128) -> Result<u128, SolverError> {
    match clock {
        0..2048 if clock.is_multiple_of(64) => Ok(64),
        2048..4096 if (clock - 2048).is_multiple_of(128) => Ok(128),
        _ => Err(SolverError::InvalidClock),
    }
}

#[cfg(feature = "n512-m512-temporal-h32")]
pub fn step(clock: u128) -> Result<u128, SolverError> {
    match clock {
        0..2048 if clock.is_multiple_of(32) => Ok(32),
        2048..4096 if (clock - 2048).is_multiple_of(64) => Ok(64),
        _ => Err(SolverError::InvalidClock),
    }
}

#[cfg(feature = "n512-m512-temporal-h16")]
pub fn step(clock: u128) -> Result<u128, SolverError> {
    match clock {
        0..2048 if clock.is_multiple_of(16) => Ok(16),
        2048..4096 if (clock - 2048).is_multiple_of(32) => Ok(32),
        _ => Err(SolverError::InvalidClock),
    }
}

pub fn validate() -> Result<(), SolverError> {
    if terminal_clock()? != ENDPOINT
        || FINE[0] != 0
        || FINE[FINE.len() - 1] != ENDPOINT
        || !nested(&MIDDLE, &FINE)
        || !nested(&COARSE, &MIDDLE)
        || !FINE.iter().copied().all(accepted_clock)
        || !simpson(&FINE)
        || !simpson(&MIDDLE)
        || !simpson(&COARSE)
    {
        return Err(SolverError::InvalidClock);
    }
    Ok(())
}

fn terminal_clock() -> Result<u128, SolverError> {
    let mut clock = 0_u128;
    for _ in 0..MAXIMUM_ATTEMPTS {
        clock = clock
            .checked_add(step(clock)?)
            .ok_or(SolverError::SizeOverflow)?;
    }
    Ok(clock)
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
        assert_eq!(terminal_clock().unwrap(), ENDPOINT);
        assert_eq!(step(ENDPOINT), Err(SolverError::InvalidClock));
    }

    #[test]
    fn unscheduled_and_rest_clocks_do_not_trigger_positive_observation() {
        for clock in [0, step(0).unwrap(), 511, 513, 4095] {
            assert!(!positive_node(clock));
        }
    }

    #[cfg(feature = "n384-piecewise-common")]
    #[test]
    fn piecewise_transition_and_invalid_clocks_are_exact() {
        assert_eq!(step(0), Ok(64));
        assert_eq!(step(1984), Ok(64));
        assert_eq!(step(2048), Ok(128));
        assert_eq!(step(3968), Ok(128));
        for clock in [1, 2000, 2112, 4096] {
            assert_eq!(step(clock), Err(SolverError::InvalidClock));
        }
        let mut clock = 0;
        let mut reached = vec![clock];
        for _ in 0..MAXIMUM_ATTEMPTS {
            clock += step(clock).unwrap();
            reached.push(clock);
        }
        assert_eq!(reached.len(), 49);
        assert_eq!(reached[32], 2048);
        assert!(FINE.iter().all(|clock| reached.contains(clock)));
    }

    #[cfg(feature = "n512-m512-temporal-h32")]
    #[test]
    fn temporal_h32_transition_lands_on_every_observer_node() {
        assert_eq!(step(0), Ok(32));
        assert_eq!(step(2016), Ok(32));
        assert_eq!(step(2048), Ok(64));
        assert_eq!(step(4032), Ok(64));
        for clock in [1, 33, 2080, 4096] {
            assert_eq!(step(clock), Err(SolverError::InvalidClock));
        }
        let mut clock = 0;
        let mut reached = vec![clock];
        for _ in 0..MAXIMUM_ATTEMPTS {
            clock += step(clock).unwrap();
            reached.push(clock);
        }
        assert_eq!(reached.len(), 97);
        assert_eq!(reached[64], 2048);
        assert_eq!(reached[96], ENDPOINT);
        assert!(FINE.iter().all(|clock| reached.contains(clock)));
        validate().unwrap();
    }

    #[cfg(feature = "n512-m512-temporal-h16")]
    #[test]
    fn temporal_h16_transition_lands_on_every_observer_node() {
        assert_eq!(step(0), Ok(16));
        assert_eq!(step(2032), Ok(16));
        assert_eq!(step(2048), Ok(32));
        assert_eq!(step(4064), Ok(32));
        for clock in [1, 17, 2064, 4096] {
            assert_eq!(step(clock), Err(SolverError::InvalidClock));
        }
        let mut clock = 0;
        let mut reached = vec![clock];
        for _ in 0..MAXIMUM_ATTEMPTS {
            clock += step(clock).unwrap();
            reached.push(clock);
        }
        assert_eq!(reached.len(), 193);
        assert_eq!(reached[128], 2048);
        assert_eq!(reached[192], ENDPOINT);
        assert!(FINE.iter().all(|clock| reached.contains(clock)));
        validate().unwrap();
    }
}
