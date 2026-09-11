//! Exact binary64 statistics/magnitudes bound to the entire imported sample packet.
use super::{input::Packet, reduce::Group, AuditError};
use nsbu_solver::diagnostics::local::LocalError;
use std::io::Write;
/// Fixed output header is 488 bytes, followed by six error/reference magnitude row pairs.
/// Every float is emitted as its original little-endian bits; no decimal rounding intervenes.
pub fn write(out: &mut impl Write, packet: &Packet, groups: &[Group; 6]) -> Result<(), AuditError> {
    out.write_all(b"NSBUMAG1")?;
    out.write_all(&(packet.plan.n as u64).to_le_bytes())?;
    out.write_all(&(packet.plan.points as u64).to_le_bytes())?;
    out.write_all(&packet.digest)?;
    for value in packet.floors {
        out.write_all(&value.to_bits().to_le_bytes())?;
    }
    for group in groups {
        statistics(out, group.direct)?;
        statistics(out, group.physical)?;
    }
    for group in groups {
        for values in [&group.error, &group.reference] {
            for value in values {
                out.write_all(&value.to_bits().to_le_bytes())?;
            }
        }
    }
    Ok(())
}
fn statistics(out: &mut impl Write, value: LocalError) -> Result<(), std::io::Error> {
    for scalar in [
        value.rms_error,
        value.peak_error,
        value.peak_relative_error,
        value.reference_peak,
    ] {
        out.write_all(&scalar.to_bits().to_le_bytes())?;
    }
    Ok(())
}
#[test]
fn full_output_binds_source_floors_statistics_and_every_magnitude_without_roundtrips() {
    let packet = Packet::read(&mut &super::input::fixture()[..], super::CAP).unwrap();
    let groups = super::reduce::all(&packet).unwrap();
    let mut out = Vec::new();
    write(&mut out, &packet, &groups).unwrap();
    assert_eq!(out.len(), 488 + 12 * 512 * 8);
    assert_eq!(&out[..8], b"NSBUMAG1");
    assert_eq!(&out[24..56], &packet.digest);
    assert_eq!(&out[56..64], &packet.floors[0].to_bits().to_le_bytes());
    assert_eq!(
        &out[104..112],
        &groups[0].direct.rms_error.to_bits().to_le_bytes()
    );
    assert_eq!(
        &out[136..144],
        &groups[0].physical.rms_error.to_bits().to_le_bytes()
    );
    assert_eq!(&out[488..496], &groups[0].error[0].to_bits().to_le_bytes());
    let mut closed = &mut [][..];
    assert!(matches!(
        write(&mut closed, &packet, &groups),
        Err(AuditError::Io(_))
    ));
}
