use crate::pool::WorkerFailure;
use nsbu_solver::Complex64;
use sha2::{Digest, Sha256};

pub(crate) const WIDTH: usize = 3;
pub(crate) const STACK_BYTES: usize = 2 * 1024 * 1024;
pub(crate) const THREAD_ALLOWANCE: usize = 64 * 1024;
pub(crate) const ALLOCATION_ALLOWANCE: usize = 64;
pub(crate) const LENGTHS: [usize; 3] = [288, 384, 576];

pub(crate) fn fill_input(values: &mut [f64], variant: usize) {
    let mut bits = (variant as u64).wrapping_add(0x9e37_79b9_7f4a_7c15);
    for (index, value) in values.iter_mut().enumerate() {
        bits ^= bits << 13;
        bits ^= bits >> 7;
        bits ^= bits << 17;
        let fraction = ((bits >> 11) as f64) * (1.0 / ((1_u64 << 53) as f64));
        *value = 2.0 * fraction - 1.0 + (index % 17) as f64 * 1e-5;
    }
}

pub(crate) fn filled<T: Clone>(length: usize, value: T) -> Result<Vec<T>, String> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(length)
        .map_err(|_| "allocation failed")?;
    values.resize(length, value);
    Ok(values)
}

pub(crate) fn hash_real(values: &[f64]) -> String {
    let mut hash = Sha256::new();
    for value in values {
        hash.update(value.to_bits().to_le_bytes());
    }
    format!("{:x}", hash.finalize())
}

pub(crate) fn hash_complex(values: &[Complex64]) -> String {
    let mut hash = Sha256::new();
    for value in values {
        hash.update(value.re.to_bits().to_le_bytes());
        hash.update(value.im.to_bits().to_le_bytes());
    }
    format!("{:x}", hash.finalize())
}

pub(crate) fn debug(error: impl std::fmt::Debug) -> String {
    format!("{error:?}")
}

pub(crate) fn worker_debug(error: WorkerFailure) -> String {
    format!("{error:?}")
}
