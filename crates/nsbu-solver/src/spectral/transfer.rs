//! Normalization-preserving strict-band padding and cropping.
use crate::{domain::Layout, Complex64, SolverError};

/// Copy the common strict band and zero every other destination coefficient.
/// Input/output lengths are validated before writing. No amplitude rescaling occurs.
/// Source Nyquist entries are omitted; checkpoints must be validated separately.
pub fn transfer(
    source: Layout,
    target: Layout,
    input: &[Complex64],
    output: &mut [Complex64],
) -> Result<(), SolverError> {
    if input.len() != source.half_len() || output.len() != target.half_len() {
        return Err(SolverError::InvalidPayload);
    }
    transfer_validated(source, target, input, output);
    Ok(())
}

/// Private workspaces have matching lengths established at construction.
pub(crate) fn transfer_validated(
    source: Layout,
    target: Layout,
    input: &[Complex64],
    output: &mut [Complex64],
) {
    output.fill(Complex64::new(0.0, 0.0));
    let [nx, ny, nz] = target.dimensions();
    for i in 0..nx {
        for j in 0..ny {
            if i == nx / 2 || j == ny / 2 {
                continue;
            }
            for k in 0..nz / 2 {
                let mode = [
                    crate::domain::signed_index(i, nx),
                    crate::domain::signed_index(j, ny),
                    k as isize,
                ];
                if let Ok((index, _)) = source.locate(mode) {
                    output[(i * ny + j) * (nz / 2 + 1) + k] = input[index];
                }
            }
        }
    }
}
