//! Bounded little-endian binary64 samples in fixed CM/HO and complete derivative order.
use super::AuditError;
use nsbu_solver::SolverError;
use sha2::{Digest, Sha256};
use std::io::Read;
pub const HEADER: usize = 136;
pub const ROWS: usize = 46;
/// Full simultaneous packet/magnitude storage and conservative metadata allowance.
#[derive(Debug, Clone, Copy)]
pub struct Plan {
    pub n: usize,
    pub points: usize,
    pub packet_bytes: usize,
    pub reserved_bytes: usize,
}
impl Plan {
    pub fn new(n: usize, cap: usize) -> Result<Self, SolverError> {
        if ![4, 8, 12].contains(&n) {
            return Err(SolverError::InvalidPayload);
        }
        let points = (2 * n).pow(3);
        let packet_bytes = HEADER + 2 * ROWS * points * 8;
        let reserved_bytes = packet_bytes + 12 * points * 8 + 1024 * 1024;
        if reserved_bytes > cap {
            return Err(SolverError::ResourceLimit);
        }
        Ok(Self {
            n,
            points,
            packet_bytes,
            reserved_bytes,
        })
    }
}
/// Imported words and a digest of the entire validated packet; no trajectory authentication.
pub struct Packet {
    pub plan: Plan,
    pub floors: [f64; 6],
    pub digest: [u8; 32],
    data: Vec<u8>,
}
impl Packet {
    /// Read a fixed stack header, preflight, then read exactly one bounded packet and EOF.
    pub fn read(reader: &mut impl Read, cap: usize) -> Result<Self, AuditError> {
        let mut header = [0; HEADER];
        reader.read_exact(&mut header)?;
        if &header[..8] != b"NSBURED1" {
            return Err(SolverError::InvalidPayload.into());
        }
        let n = usize::try_from(word(&header, 8)).map_err(|_| SolverError::SizeOverflow)?;
        let plan = Plan::new(n, cap)?;
        if word(&header, 16) != plan.points as u64 {
            return Err(SolverError::InvalidPayload.into());
        }
        let floors = std::array::from_fn(|i| f64::from_bits(word(&header, 24 + 8 * i)));
        if floors.iter().any(|x| !x.is_finite() || *x <= 0.0) {
            return Err(SolverError::InvalidPayload.into());
        }
        let mut data = vec![0; plan.packet_bytes];
        data[..HEADER].copy_from_slice(&header);
        reader.read_exact(&mut data[HEADER..])?;
        if reader.read(&mut [0])? != 0 {
            return Err(SolverError::InvalidPayload.into());
        }
        for offset in (HEADER..data.len()).step_by(8) {
            if !f64::from_bits(word(&data, offset)).is_finite() {
                return Err(SolverError::InvalidPayload.into());
            }
        }
        let digest = Sha256::digest(&data).into();
        Ok(Self {
            plan,
            floors,
            digest,
            data,
        })
    }
    /// Internal indices come only from the fixed row inventory and admitted point loop.
    pub fn value(&self, role: usize, row: usize, point: usize) -> f64 {
        let offset = HEADER + 8 * ((role * ROWS + row) * self.plan.points + point);
        f64::from_bits(word(&self.data, offset))
    }
}
fn word(data: &[u8], offset: usize) -> u64 {
    u64::from_le_bytes(
        data[offset..offset + 8]
            .try_into()
            .expect("bounded fixed-width word"),
    )
}
#[cfg(test)]
pub fn fixture() -> Vec<u8> {
    include_bytes!("../../data/reduction-n4.bin").to_vec()
}
#[test]
fn exact_words_caps_and_malformed_input_are_distinct_refusals() {
    use std::io::Cursor;
    let data = fixture();
    let p = Packet::read(&mut Cursor::new(&data), super::CAP).unwrap();
    assert_eq!(p.plan.points, 512);
    assert_eq!(p.plan.packet_bytes, data.len());
    assert_eq!(p.digest, <[u8; 32]>::from(Sha256::digest(&data)));
    for n in [0, 3, 16, usize::MAX] {
        assert!(Plan::new(n, usize::MAX).is_err());
    }
    assert!(Plan::new(12, 1).is_err());
    assert!(Packet::read(&mut Cursor::new(&data), 1).is_err());
    for invalid in [&data[..0], &data[..135], &data[..data.len() - 1]] {
        assert!(Packet::read(&mut Cursor::new(invalid), super::CAP).is_err());
    }
    let mut extra = data.clone();
    extra.push(0);
    assert!(Packet::read(&mut Cursor::new(extra), super::CAP).is_err());
    for (offset, bits) in [
        (0, 0),
        (8, 3),
        (16, 513),
        (24, 0),
        (24, f64::INFINITY.to_bits()),
        (HEADER, f64::NAN.to_bits()),
    ] {
        let mut bad = data.clone();
        bad[offset..offset + 8].copy_from_slice(&bits.to_le_bytes());
        assert!(Packet::read(&mut Cursor::new(bad), super::CAP).is_err());
    }
    let mut signed = data;
    signed[HEADER..HEADER + 8].copy_from_slice(&(-0.0_f64).to_bits().to_le_bytes());
    let p = Packet::read(&mut Cursor::new(signed), super::CAP).unwrap();
    assert_eq!(p.value(0, 0, 0).to_bits(), (-0.0_f64).to_bits());
}
