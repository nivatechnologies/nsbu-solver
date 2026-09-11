//! Disjoint axial-plane samples; each scalar root is computed once over the whole pool.
use crate::{fields::axial, time::BenchmarkTime};
use nsbu_solver::{domain::Layout, SolverError};
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
pub(super) struct Samples {
    pub physical: [Vec<f64>; 3],
    roots: Vec<Option<axial::AxialRoot>>,
}
impl Samples {
    pub fn new(partition: Partition) -> Result<Self, SolverError> {
        let n = partition.points();
        Ok(Self {
            physical: [
                super::super::buffer(n, 0.0)?,
                super::super::buffer(n, 0.0)?,
                super::super::buffer(n, 0.0)?,
            ],
            roots: super::super::buffer(partition.planes, None)?,
        })
    }
    pub fn evaluate(
        &mut self,
        partition: Partition,
        time: BenchmarkTime,
    ) -> Result<usize, SolverError> {
        let [nx, ny, nz] = partition.layout.dimensions();
        let mut iterations = 0;
        for (plane, slot) in self.roots.iter_mut().enumerate() {
            *slot = axial::AxialRoot::new(partition.z_index(plane) as f64 / nz as f64, time)
                .map_err(|_| SolverError::ArithmeticResolutionLimited)?;
            iterations += slot.map_or(0, axial::AxialRoot::iterations);
        }
        for i in 0..nx {
            for j in 0..ny {
                for plane in 0..partition.planes {
                    let point = [
                        i as f64 / nx as f64,
                        j as f64 / ny as f64,
                        partition.z_index(plane) as f64 / nz as f64,
                    ];
                    let value = axial::evaluate(point, time, self.roots[plane].as_ref())
                        .map_err(|_| SolverError::ArithmeticResolutionLimited)?;
                    let index = (i * ny + j) * partition.planes + plane;
                    for (values, force) in self.physical.iter_mut().zip(value.force) {
                        values[index] = force;
                    }
                }
            }
        }
        Ok(iterations)
    }
    pub fn copy_to(&self, partition: Partition, target: &mut [Vec<f64>; 3]) {
        let nz = partition.layout.dimensions()[2];
        for (local, global) in self.physical.iter().zip(target) {
            for (index, &value) in local.iter().enumerate() {
                let row = index / partition.planes;
                let plane = index % partition.planes;
                global[row * nz + partition.z_index(plane)] = value;
            }
        }
    }
}
