//! Fallible initial allocations shared by planning and state construction.
use crate::{Complex64, SolverError};

pub(crate) fn filled<T: Clone>(count: usize, value: T) -> Result<Vec<T>, SolverError> {
    let mut values = reserved(count)?;
    values.resize(count, value);
    Ok(values)
}

pub(crate) fn reserved<T>(count: usize) -> Result<Vec<T>, SolverError> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(count)
        .map_err(|_| SolverError::AllocationFailed)?;
    Ok(values)
}

pub(crate) fn field(n: usize) -> Result<[Vec<Complex64>; 3], SolverError> {
    let zero = Complex64::new(0.0, 0.0);
    Ok([filled(n, zero)?, filled(n, zero)?, filled(n, zero)?])
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Complex64;

    #[test]
    fn impossible_reservation_returns_a_typed_error() {
        assert_eq!(
            filled(usize::MAX, Complex64::new(0.0, 0.0)),
            Err(SolverError::AllocationFailed)
        );
    }
}
