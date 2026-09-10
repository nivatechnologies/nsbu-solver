//! Shared geometry traversal; callers validate their borrowed coefficient shapes first.
use crate::{domain::Domain, spectral::modal, SolverError};

pub(super) struct RetainedMode {
    pub index: usize,
    pub integer: [isize; 3],
    pub wave: [f64; 3],
    pub weight: f64,
}

pub(super) fn visit(
    domain: Domain,
    operation: &mut dyn FnMut(RetainedMode) -> Result<(), SolverError>,
) -> Result<(), SolverError> {
    let layout = domain.layout();
    for index in 0..layout.half_len() {
        let position = layout.position(index)?;
        if layout.is_nyquist(position)? {
            continue;
        }
        let integer = layout.mode(position)?;
        operation(RetainedMode {
            index,
            integer,
            wave: modal::wavevector(domain, integer)?,
            weight: layout.weight(position)?,
        })?;
    }
    Ok(())
}
