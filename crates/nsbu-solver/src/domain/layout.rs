//! Last-axis-contiguous real-to-complex half-spectrum indexing.
use crate::SolverError;

/// An even physical grid and its checked half-spectrum element count.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Layout {
    dimensions: [usize; 3],
    real_len: usize,
    half_len: usize,
}

impl Layout {
    /// Construct an even grid, including the six-multiple padded grids.
    pub fn new(dimensions: [usize; 3]) -> Result<Self, SolverError> {
        if dimensions.iter().any(|&n| n < 2 || !n.is_multiple_of(2)) {
            return Err(SolverError::InvalidDomain);
        }
        let [nx, ny, nz] = dimensions;
        let plane = nx.checked_mul(ny).ok_or(SolverError::SizeOverflow)?;
        let real_len = plane.checked_mul(nz).ok_or(SolverError::SizeOverflow)?;
        let half_len = plane
            .checked_mul(nz / 2 + 1)
            .ok_or(SolverError::SizeOverflow)?;
        // A complex coefficient occupies 16 bytes; even one buffer must be addressable.
        if half_len > isize::MAX as usize / 16 {
            return Err(SolverError::SizeOverflow);
        }
        Ok(Self {
            dimensions,
            real_len,
            half_len,
        })
    }

    /// Physical grid dimensions.
    pub fn dimensions(self) -> [usize; 3] {
        self.dimensions
    }

    /// Number of real grid points.
    pub fn real_len(self) -> usize {
        self.real_len
    }

    /// Number of stored complex coefficients per component.
    pub fn half_len(self) -> usize {
        self.half_len
    }

    /// Checked flattened storage index, with contiguous last index.
    pub fn index(self, position: [usize; 3]) -> Result<usize, SolverError> {
        let [i, j, k] = position;
        let [nx, ny, nz] = self.dimensions;
        if i >= nx || j >= ny || k > nz / 2 {
            return Err(SolverError::InvalidIndex);
        }
        Ok((i * ny + j) * (nz / 2 + 1) + k)
    }

    /// Signed integer mode associated with a stored index.
    pub fn mode(self, position: [usize; 3]) -> Result<[isize; 3], SolverError> {
        self.index(position)?;
        Ok(std::array::from_fn(|axis| {
            let i = position[axis];
            let n = self.dimensions[axis];
            if i > n / 2 {
                i as isize - n as isize
            } else {
                i as isize
            }
        }))
    }

    /// Whether a valid position lies on any excluded Nyquist plane.
    pub fn is_nyquist(self, position: [usize; 3]) -> Result<bool, SolverError> {
        self.index(position)?;
        Ok(position
            .iter()
            .zip(self.dimensions)
            .any(|(&i, n)| i == n / 2))
    }

    /// Half-spectrum Parseval multiplicity. Excluded Nyquist entries have zero weight.
    pub fn weight(self, position: [usize; 3]) -> Result<f64, SolverError> {
        if self.is_nyquist(position)? {
            return Ok(0.0);
        }
        Ok(if position[2] == 0 { 1.0 } else { 2.0 })
    }

    /// Locate a signed mode; negative last-axis modes require conjugation.
    /// Only the strict retained band is accepted, so Nyquist ambiguity is refused.
    pub fn locate(self, mode: [isize; 3]) -> Result<(usize, bool), SolverError> {
        if mode
            .iter()
            .zip(self.dimensions)
            .any(|(&m, n)| m.unsigned_abs() >= n / 2)
        {
            return Err(SolverError::InvalidIndex);
        }
        let conjugate = mode[2] < 0;
        let position = std::array::from_fn(|axis| {
            let m = if conjugate { -mode[axis] } else { mode[axis] };
            m.rem_euclid(self.dimensions[axis] as isize) as usize
        });
        Ok((self.index(position)?, conjugate))
    }
}
