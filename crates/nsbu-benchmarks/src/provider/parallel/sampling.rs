//! Disjoint axial-plane samples; each scalar root is computed once over the whole pool.
use crate::{fields::axial, reduced_force::axial as reduced_axial, time::BenchmarkTime};
use nsbu_solver::{domain::Layout, SolverError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(in crate::provider) enum Arithmetic {
    Cartesian,
    Reduced,
}

#[derive(Debug, Clone, Copy)]
pub(super) struct Partition {
    pub layout: Layout,
    pub worker: usize,
    pub workers: usize,
    pub planes: usize,
}
impl Partition {
    pub fn new(layout: Layout, worker: usize, workers: usize) -> Self {
        Self {
            layout,
            worker,
            workers,
            planes: (layout.dimensions()[2] - worker).div_ceil(workers),
        }
    }
    pub fn points(self) -> usize {
        let [nx, ny, _] = self.layout.dimensions();
        nx * ny * self.planes
    }
    pub fn z_index(self, plane: usize) -> usize {
        self.worker + plane * self.workers
    }
}
pub(super) enum Samples {
    Cartesian {
        physical: [Vec<f64>; 3],
        roots: Vec<Option<axial::AxialRoot>>,
    },
    Reduced {
        physical: [Vec<f64>; 3],
        roots: Vec<Option<reduced_axial::AxialRoot>>,
    },
}
impl Samples {
    pub fn new(partition: Partition, arithmetic: Arithmetic) -> Result<Self, SolverError> {
        let n = partition.points();
        let physical = [
            super::super::buffer(n, 0.0)?,
            super::super::buffer(n, 0.0)?,
            super::super::buffer(n, 0.0)?,
        ];
        match arithmetic {
            Arithmetic::Cartesian => Ok(Self::Cartesian {
                physical,
                roots: super::super::buffer(partition.planes, None)?,
            }),
            Arithmetic::Reduced => Ok(Self::Reduced {
                physical,
                roots: super::super::buffer(partition.planes, None)?,
            }),
        }
    }
    pub fn evaluate(
        &mut self,
        partition: Partition,
        time: BenchmarkTime,
    ) -> Result<usize, SolverError> {
        match self {
            Self::Cartesian { physical, roots } => {
                evaluate_cartesian(physical, roots, partition, time)
            }
            Self::Reduced { physical, roots } => evaluate_reduced(physical, roots, partition, time),
        }
    }
    pub fn copy_to(&self, partition: Partition, target: &mut [Vec<f64>; 3]) {
        let physical = match self {
            Self::Cartesian { physical, .. } | Self::Reduced { physical, .. } => physical,
        };
        let nz = partition.layout.dimensions()[2];
        for (local, global) in physical.iter().zip(target) {
            for (index, &value) in local.iter().enumerate() {
                let row = index / partition.planes;
                let plane = index % partition.planes;
                global[row * nz + partition.z_index(plane)] = value;
            }
        }
    }
}

fn evaluate_cartesian(
    physical: &mut [Vec<f64>; 3],
    roots: &mut [Option<axial::AxialRoot>],
    partition: Partition,
    time: BenchmarkTime,
) -> Result<usize, SolverError> {
    let [nx, ny, nz] = partition.layout.dimensions();
    let mut iterations = 0;
    for (plane, slot) in roots.iter_mut().enumerate() {
        *slot = axial::AxialRoot::new(partition.z_index(plane) as f64 / nz as f64, time)
            .map_err(|_| SolverError::ArithmeticResolutionLimited)?;
        iterations += slot.map_or(0, axial::AxialRoot::iterations);
    }
    for i in 0..nx {
        for j in 0..ny {
            for (plane, root) in roots.iter().enumerate() {
                let point = [
                    i as f64 / nx as f64,
                    j as f64 / ny as f64,
                    partition.z_index(plane) as f64 / nz as f64,
                ];
                let value = axial::evaluate(point, time, root.as_ref())
                    .map_err(|_| SolverError::ArithmeticResolutionLimited)?;
                let index = (i * ny + j) * partition.planes + plane;
                for (values, force) in physical.iter_mut().zip(value.force) {
                    values[index] = force;
                }
            }
        }
    }
    Ok(iterations)
}

fn evaluate_reduced(
    physical: &mut [Vec<f64>; 3],
    roots: &mut [Option<reduced_axial::AxialRoot>],
    partition: Partition,
    time: BenchmarkTime,
) -> Result<usize, SolverError> {
    let [nx, ny, nz] = partition.layout.dimensions();
    let mut iterations = 0;
    for (plane, slot) in roots.iter_mut().enumerate() {
        *slot = reduced_axial::AxialRoot::new(partition.z_index(plane) as f64 / nz as f64, time)
            .map_err(|_| SolverError::ArithmeticResolutionLimited)?;
        iterations += slot.map_or(0, reduced_axial::AxialRoot::iterations);
    }
    for i in 0..nx {
        for j in 0..ny {
            for (plane, root) in roots.iter().enumerate() {
                let point = [
                    i as f64 / nx as f64,
                    j as f64 / ny as f64,
                    partition.z_index(plane) as f64 / nz as f64,
                ];
                let value = reduced_axial::evaluate(point, time, root.as_ref())
                    .map_err(|_| SolverError::ArithmeticResolutionLimited)?;
                let index = (i * ny + j) * partition.planes + plane;
                for (values, force) in physical.iter_mut().zip(value.force) {
                    values[index] = force;
                }
            }
        }
    }
    Ok(iterations)
}
