//! Padding preserves normalized amplitudes and all strict modes without a two-thirds cut.
use nsbu_solver::domain::Layout;
use nsbu_solver::spectral::transfer;
use nsbu_solver::{Complex64, SolverError};

#[test]
fn anisotropic_pad_crop_roundtrip_preserves_upper_band_and_mean() {
    let small = Layout::new([8, 4, 12]).unwrap();
    let padded = Layout::new([12, 6, 18]).unwrap();
    let mut original = vec![Complex64::new(0.0, 0.0); small.half_len()];
    let modes = [[0, 0, 0], [-3, 1, 5], [3, -1, 4], [1, 0, 0], [-1, 0, 0]];
    for (index, mode) in modes.into_iter().enumerate() {
        original[small.locate(mode).unwrap().0] = Complex64::new(index as f64 + 1.0, 0.0);
    }
    let mut large = vec![Complex64::new(999.0, 2.0); padded.half_len()];
    transfer(small, padded, &original, &mut large).unwrap();
    for mode in modes {
        assert_eq!(
            large[padded.locate(mode).unwrap().0],
            original[small.locate(mode).unwrap().0]
        );
    }
    assert_eq!(
        large[padded.locate([5, 2, 8]).unwrap().0],
        Complex64::new(0.0, 0.0)
    );
    large[padded.locate([5, 2, 8]).unwrap().0] = Complex64::new(13.0, -4.0);
    let mut cropped = vec![Complex64::new(111.0, 0.0); small.half_len()];
    transfer(padded, small, &large, &mut cropped).unwrap();
    assert_eq!(cropped, original);
}

#[test]
fn nyquist_is_omitted_and_invalid_buffers_leave_output_unchanged() {
    let layout = Layout::new([4; 3]).unwrap();
    let mut source = vec![Complex64::new(0.0, 0.0); 48];
    for position in [[2, 0, 0], [0, 2, 0], [0, 0, 2]] {
        source[layout.index(position).unwrap()] = Complex64::new(4.0, 0.0);
    }
    let mut output = vec![Complex64::new(1.0, 0.0); 48];
    assert_eq!(
        transfer(layout, layout, &source[..47], &mut output),
        Err(SolverError::InvalidPayload)
    );
    assert_eq!(output, vec![Complex64::new(1.0, 0.0); 48]);
    assert_eq!(
        transfer(layout, layout, &source, &mut output[..47]),
        Err(SolverError::InvalidPayload)
    );
    transfer(layout, layout, &source, &mut output).unwrap();
    assert_eq!(output, vec![Complex64::new(0.0, 0.0); 48]);
}
