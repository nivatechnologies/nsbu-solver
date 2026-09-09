//! Exhaustive small anisotropic indexing, conjugacy and Parseval multiplicities.
use nsbu_solver::domain::{Domain, Layout};
use nsbu_solver::SolverError;

#[test]
fn storage_order_signed_modes_and_negative_last_axis() {
    let layout = Layout::new([4, 8, 12]).unwrap();
    assert_eq!(layout.dimensions(), [4, 8, 12]);
    assert_eq!(layout.real_len(), 384);
    assert_eq!(layout.half_len(), 224);
    for i in 0..4 {
        for j in 0..8 {
            for k in 0..7 {
                let position = [i, j, k];
                assert_eq!(layout.index(position), Ok((i * 8 + j) * 7 + k));
                assert_eq!(layout.position((i * 8 + j) * 7 + k), Ok(position));
                let mode = layout.mode(position).unwrap();
                assert_eq!(
                    mode,
                    [
                        if i > 2 { i as isize - 4 } else { i as isize },
                        if j > 4 { j as isize - 8 } else { j as isize },
                        k as isize
                    ]
                );
            }
        }
    }
    assert_eq!(
        layout.locate([1, -3, -5]),
        Ok((layout.index([3, 3, 5]).unwrap(), true))
    );
    assert_eq!(
        layout.locate([-1, -3, 5]),
        Ok((layout.index([3, 5, 5]).unwrap(), false))
    );
    assert_eq!(layout.locate([0, 0, 0]), Ok((0, false)));
    assert_eq!(layout.position(224), Err(SolverError::InvalidIndex));
    assert_eq!(layout.position(usize::MAX), Err(SolverError::InvalidIndex));
}

#[test]
fn strict_band_and_half_spectrum_multiplicities() {
    let layout = Layout::new([4, 8, 12]).unwrap();
    assert_eq!(layout.weight([1, 1, 0]), Ok(1.0));
    assert_eq!(layout.weight([1, 1, 1]), Ok(2.0));
    for position in [[2, 0, 0], [0, 4, 0], [0, 0, 6]] {
        assert_eq!(layout.is_nyquist(position), Ok(true));
        assert_eq!(layout.weight(position), Ok(0.0));
    }
    for mode in [
        [2, 0, 0],
        [-2, 0, 0],
        [0, 4, 0],
        [0, -4, 0],
        [0, 0, 6],
        [0, 0, -6],
        [isize::MIN, 0, 0],
    ] {
        assert_eq!(layout.locate(mode), Err(SolverError::InvalidIndex));
    }
    for position in [[4, 0, 0], [0, 8, 0], [0, 0, 7], [usize::MAX, 0, 0]] {
        assert_eq!(layout.index(position), Err(SolverError::InvalidIndex));
        assert_eq!(layout.mode(position), Err(SolverError::InvalidIndex));
        assert_eq!(layout.weight(position), Err(SolverError::InvalidIndex));
    }
}

#[test]
fn invalid_and_unaddressable_layouts_are_refused_before_allocation() {
    for dims in [
        [0, 4, 4],
        [4, 0, 4],
        [4, 4, 0],
        [1, 4, 4],
        [4, 3, 4],
        [4, 4, 5],
    ] {
        assert_eq!(Layout::new(dims), Err(SolverError::InvalidDomain));
    }
    for dims in [
        [usize::MAX - 1, 4, 4],
        [4, 4, usize::MAX - 1],
        [2, 2, (isize::MAX as usize / 8) + 1],
    ] {
        assert_eq!(Layout::new(dims), Err(SolverError::SizeOverflow));
    }
    assert_eq!(Layout::new([2, 2, 2]).unwrap().half_len(), 8);
}

#[test]
fn domain_preserves_positive_geometry_and_pads_all_axes() {
    let domain = Domain::new([4, 8, 12], [1.0, 2.0, 3.0], 0.25).unwrap();
    assert_eq!(domain.lengths(), [1.0, 2.0, 3.0]);
    assert_eq!(domain.viscosity(), 0.25);
    assert_eq!(domain.layout().dimensions(), [4, 8, 12]);
    assert_eq!(domain.padded_layout().unwrap().dimensions(), [6, 12, 18]);
    for value in [0.0, -1.0, f64::NAN, f64::INFINITY, f64::NEG_INFINITY] {
        assert_eq!(
            Domain::new([4; 3], [1.0; 3], value),
            Err(SolverError::InvalidDomain)
        );
        for axis in 0..3 {
            let mut lengths = [1.0; 3];
            lengths[axis] = value;
            assert_eq!(
                Domain::new([4; 3], lengths, 1.0),
                Err(SolverError::InvalidDomain)
            );
        }
    }
    for dims in [[0, 4, 4], [4, 6, 4], [4, 4, 2]] {
        assert_eq!(
            Domain::new(dims, [1.0; 3], 1.0),
            Err(SolverError::InvalidDomain)
        );
    }
    assert_eq!(
        Domain::new([usize::MAX - 3, 4, 4], [1.0; 3], 1.0),
        Err(SolverError::SizeOverflow)
    );
}
