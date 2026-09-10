//! Fixed-width little-endian node codec; sizes are admitted by the enclosing reader.
use super::{Node, TickClock};
use nsbu_solver::{
    checkpoint::CheckpointError,
    domain::{validate_spectrum, Domain, Epoch},
    Complex64,
};
pub(super) fn put(output: &mut [u8], p: &mut usize, bytes: &[u8]) {
    let end = *p + bytes.len();
    output[*p..end].copy_from_slice(bytes);
    *p = end;
}
pub(super) fn write_node(node: &Node, output: &mut [u8], p: &mut usize) {
    let clock = node.clock.expect("accepted node has a clock");
    put(output, p, &clock.exponent().to_le_bytes());
    for value in [
        clock.target(),
        clock.elapsed(),
        clock.remaining(),
        node.epoch.0,
        node.steps,
    ] {
        put(output, p, &value.to_le_bytes());
    }
    for field in node.value.iter().chain(node.derivative.iter()) {
        for value in field {
            put(output, p, &value.re.to_bits().to_le_bytes());
            put(output, p, &value.im.to_bits().to_le_bytes());
        }
    }
}
pub(super) struct Cursor<'a>(pub(super) &'a [u8]);
impl Cursor<'_> {
    pub(super) fn array<const N: usize>(&mut self) -> Result<[u8; N], CheckpointError> {
        let (head, tail) = self
            .0
            .split_at_checked(N)
            .ok_or(CheckpointError::InvalidEncoding)?;
        let mut result = [0; N];
        result.copy_from_slice(head);
        self.0 = tail;
        Ok(result)
    }
    pub(super) fn size(&mut self) -> Result<usize, CheckpointError> {
        usize::try_from(u128::from_le_bytes(self.array()?))
            .map_err(|_| CheckpointError::ResourceLimit)
    }
    pub(super) fn clock(&mut self) -> Result<TickClock, CheckpointError> {
        TickClock::restore(
            i32::from_le_bytes(self.array()?),
            u128::from_le_bytes(self.array()?),
            u128::from_le_bytes(self.array()?),
            u128::from_le_bytes(self.array()?),
        )
        .map_err(CheckpointError::InvalidHistory)
    }
}
pub(super) fn read_node(
    c: &mut Cursor<'_>,
    domain: Domain,
    node: &mut Node,
) -> Result<(), CheckpointError> {
    node.clock = Some(c.clock()?);
    node.epoch = Epoch(u128::from_le_bytes(c.array()?));
    node.steps = u128::from_le_bytes(c.array()?);
    for field in node.value.iter_mut().chain(node.derivative.iter_mut()) {
        for value in field.iter_mut() {
            *value = Complex64::new(
                f64::from_bits(u64::from_le_bytes(c.array()?)),
                f64::from_bits(u64::from_le_bytes(c.array()?)),
            );
        }
        validate_spectrum(domain.layout(), field, 1e-12)
            .map_err(CheckpointError::InvalidHistory)?;
    }
    Ok(())
}
