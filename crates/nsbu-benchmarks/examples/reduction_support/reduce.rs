//! Both production TensorErrors entry paths, with exact rounded magnitude words retained.
use super::input::Packet;
use nsbu_solver::{
    diagnostics::local::{LocalError, SampledError, TensorErrors},
    SolverError,
};
/// All four reported statistics for each independent entry path, plus per-point magnitudes.
pub struct Group {
    pub direct: LocalError,
    pub physical: LocalError,
    pub error: Vec<f64>,
    pub reference: Vec<f64>,
}
/// Complete canonical six-quantity report. The caller publishes only after every group succeeds.
pub fn all(packet: &Packet) -> Result<[Group; 6], SolverError> {
    Ok([
        group(packet, [0, 13, 26], 0)?,
        group(packet, [1, 2, 3, 14, 15, 16, 27, 28, 29], 1)?,
        group(
            packet,
            [
                4, 5, 6, 7, 8, 9, 10, 11, 12, 17, 18, 19, 20, 21, 22, 23, 24, 25, 30, 31, 32, 33,
                34, 35, 36, 37, 38,
            ],
            2,
        )?,
        group(packet, [39, 40, 41], 3)?,
        group(packet, [42], 4)?,
        group(packet, [43, 44, 45], 5)?,
    ])
}
fn group<const C: usize>(
    packet: &Packet,
    rows: [usize; C],
    quantity: usize,
) -> Result<Group, SolverError> {
    let points = packet.plan.points;
    let mut direct = TensorErrors::<C>::new(points, packet.floors[quantity])?;
    let mut physical = direct;
    let mut error = vec![0.0_f64; points];
    let mut reference = vec![0.0_f64; points];
    for point in 0..points {
        let left = rows.map(|row| packet.value(0, row, point));
        let right = rows.map(|row| packet.value(1, row, point));
        // Match the physical workspace's sequential full-component hypot order.
        for component in 0..C {
            error[point] = error[point].hypot(left[component] - right[component]);
            reference[point] = reference[point].hypot(right[component]);
        }
        direct.push(left, right)?;
        physical.push_magnitudes(error[point], reference[point])?;
    }
    Ok(Group {
        direct: measured(direct.finish()?)?,
        physical: measured(physical.finish()?)?,
        error,
        reference,
    })
}
fn measured(value: SampledError) -> Result<LocalError, SolverError> {
    match value {
        SampledError::Measured(result) => Ok(result),
        SampledError::NoSamples => Err(SolverError::InvalidPayload),
    }
}
#[test]
fn complete_tensor_counts_and_both_paths_retain_actual_nonzero_comparisons() {
    let packet = Packet::read(&mut &super::input::fixture()[..], super::CAP).unwrap();
    for (g, c) in all(&packet).unwrap().iter().zip([3, 9, 27, 3, 1, 3]) {
        assert_eq!(g.direct.components, c);
        assert_eq!(g.physical.components, c);
        assert_eq!(g.direct.samples, 512);
        assert_eq!(g.physical.samples, 512);
        assert!(g.direct.rms_error > 0.0);
        assert!(g.physical.rms_error > 0.0);
        assert_eq!(g.direct.peak_error, g.physical.peak_error);
        assert_eq!(g.direct.reference_peak, g.physical.reference_peak);
        assert_eq!(g.direct.peak_relative_error, g.physical.peak_relative_error);
        assert_eq!(g.error.len(), 512);
        assert_eq!(g.reference.len(), 512);
    }
    assert!(measured(SampledError::NoSamples).is_err());
}
#[test]
fn zero_reference_cancellation_and_overflow_do_not_hide_missing_measurements() {
    use super::input::{fixture, HEADER, ROWS};
    let mut data = fixture();
    data[HEADER..].fill(0);
    let packet = Packet::read(&mut &data[..], super::CAP).unwrap();
    for group in all(&packet).unwrap() {
        assert_eq!(group.direct.rms_error, 0.0);
        assert_eq!(group.physical.reference_peak, 0.0);
    }
    let index = |role, row| HEADER + 8 * ((role * ROWS + row) * 512);
    for role in 0..2 {
        let i = index(role, 0);
        data[i..i + 8].copy_from_slice(&1e150_f64.to_bits().to_le_bytes());
    }
    let i = index(0, 13);
    data[i..i + 8].copy_from_slice(&3e100_f64.to_bits().to_le_bytes());
    let packet = Packet::read(&mut &data[..], super::CAP).unwrap();
    let report = all(&packet).unwrap();
    assert_eq!(report[0].direct.peak_error, 3e100);
    assert_eq!(report[0].physical.reference_peak, 1e150);
    for (role, x) in [(0, f64::MAX), (1, -f64::MAX)] {
        let i = index(role, 0);
        data[i..i + 8].copy_from_slice(&x.to_bits().to_le_bytes());
    }
    let packet = Packet::read(&mut &data[..], super::CAP).unwrap();
    assert!(all(&packet).is_err());
}
