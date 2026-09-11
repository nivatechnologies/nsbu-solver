use nsbu_solver::{domain::Layout, Complex64};

pub(super) const ALLOWANCE_BYTES: usize = 8 * 1024;
const ALLOCATOR_ALLOWANCE_BYTES: usize = 3 * 64;

pub(super) struct Basis {
    dimensions: [usize; 3],
    samples: [usize; 3],
    axes: [Vec<Complex64>; 3],
}

impl Basis {
    pub(super) fn new(dimensions: [usize; 3], samples: Layout) -> Self {
        let sample_dimensions = samples.dimensions();
        let expected_bytes = reservation_bytes(dimensions, sample_dimensions)
            .expect("phase basis reservation overflow");
        assert!(expected_bytes <= ALLOWANCE_BYTES);
        let axes = std::array::from_fn(|axis| {
            let n = dimensions[axis];
            let count = n - 1;
            let mut values = Vec::with_capacity(sample_dimensions[axis] * count);
            for point in 0..sample_dimensions[axis] {
                for mode in -(n as isize / 2) + 1..n as isize / 2 {
                    values.push(axis_phase(sample_dimensions[axis], mode, point));
                }
            }
            values
        });
        let basis = Self {
            dimensions,
            samples: sample_dimensions,
            axes,
        };
        assert!(basis.storage_bytes() <= expected_bytes);
        basis
    }

    pub(super) fn phase(&self, mode: [isize; 3], point: [usize; 3]) -> Complex64 {
        let values: [Complex64; 3] = std::array::from_fn(|axis| {
            let count = self.dimensions[axis] - 1;
            let offset = (mode[axis] + self.dimensions[axis] as isize / 2 - 1) as usize;
            self.axes[axis][point[axis] * count + offset]
        });
        values[0] * values[1] * values[2]
    }

    pub(super) fn point(&self, index: [usize; 3]) -> [f64; 3] {
        std::array::from_fn(|axis| index[axis] as f64 / self.samples[axis] as f64)
    }

    fn storage_bytes(&self) -> usize {
        std::mem::size_of::<Self>()
            + ALLOCATOR_ALLOWANCE_BYTES
            + self
                .axes
                .iter()
                .map(|values| values.capacity() * std::mem::size_of::<Complex64>())
                .sum::<usize>()
    }
}

fn reservation_bytes(dimensions: [usize; 3], samples: [usize; 3]) -> Option<usize> {
    let values =
        dimensions
            .into_iter()
            .zip(samples)
            .try_fold(0usize, |total, (dimension, sample)| {
                sample
                    .checked_mul(dimension.checked_sub(1)?)?
                    .checked_add(total)
            })?;
    values
        .checked_mul(std::mem::size_of::<Complex64>())?
        .checked_add(std::mem::size_of::<Basis>())?
        .checked_add(ALLOCATOR_ALLOWANCE_BYTES)
}

fn axis_phase(samples: usize, mode: isize, point: usize) -> Complex64 {
    let angle = std::f64::consts::TAU * mode as f64 * point as f64 / samples as f64;
    let (sin, cos) = angle.sin_cos();
    Complex64::new(cos, sin)
}
