//! Allocation-free byte operations used only after container-specific size preflight.
use super::CheckpointError;
pub(super) fn put(output: &mut [u8], position: &mut usize, bytes: &[u8]) {
    let end = *position + bytes.len();
    output[*position..end].copy_from_slice(bytes);
    *position = end;
}
pub(super) struct Cursor<'a> {
    pub(super) remaining: &'a [u8],
}
impl<'a> Cursor<'a> {
    pub(super) fn take(&mut self, count: usize) -> Result<&'a [u8], CheckpointError> {
        let (head, tail) = self
            .remaining
            .split_at_checked(count)
            .ok_or(CheckpointError::InvalidEncoding)?;
        self.remaining = tail;
        Ok(head)
    }
    pub(super) fn array<const N: usize>(&mut self) -> Result<[u8; N], CheckpointError> {
        let mut bytes = [0; N];
        bytes.copy_from_slice(self.take(N)?);
        Ok(bytes)
    }
}
