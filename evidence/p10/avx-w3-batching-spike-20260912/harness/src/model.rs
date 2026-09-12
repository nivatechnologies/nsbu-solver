use crate::util::{debug, fill_input, filled, WIDTH};
use nsbu_solver::{
    domain::Layout,
    spectral::{FftCatalog, FftPlan, FftWorkspace},
    Complex64, SolverError,
};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Operation {
    Forward,
    Inverse,
    Noop,
}

pub(crate) struct Lane {
    fft: FftPlan,
    work: FftWorkspace,
    pub(crate) physical: Vec<f64>,
    pub(crate) spectrum: Vec<Complex64>,
    staged_physical: Vec<f64>,
    staged_spectrum: Vec<Complex64>,
}

impl Lane {
    pub(crate) fn new(layout: Layout, catalog: &FftCatalog) -> Result<Self, String> {
        let bytes = FftPlan::reservation_from_catalog(layout, catalog).map_err(debug)?;
        let (fft, work) = FftPlan::new_from_catalog(layout, catalog, bytes).map_err(debug)?;
        Ok(Self {
            fft,
            work,
            physical: filled(layout.real_len(), 0.0)?,
            spectrum: filled(layout.half_len(), Complex64::new(0.0, 0.0))?,
            staged_physical: filled(layout.real_len(), 0.0)?,
            staged_spectrum: filled(layout.half_len(), Complex64::new(0.0, 0.0))?,
        })
    }

    pub(crate) fn reset(&mut self, variant: usize) {
        fill_input(&mut self.physical, variant);
        self.spectrum.fill(Complex64::new(0.0, 0.0));
        self.staged_physical.fill(0.0);
        self.staged_spectrum.fill(Complex64::new(0.0, 0.0));
    }

    pub(crate) fn run(&mut self, operation: Operation) -> Result<(), SolverError> {
        match operation {
            Operation::Forward => {
                self.fft
                    .forward(&self.physical, &mut self.staged_spectrum, &mut self.work)
            }
            Operation::Inverse => {
                self.fft
                    .inverse(&self.spectrum, &mut self.staged_physical, &mut self.work)
            }
            Operation::Noop => Ok(()),
        }
    }

    pub(crate) fn publish(&mut self, operation: Operation) {
        match operation {
            Operation::Forward => std::mem::swap(&mut self.spectrum, &mut self.staged_spectrum),
            Operation::Inverse => std::mem::swap(&mut self.physical, &mut self.staged_physical),
            Operation::Noop => {}
        }
    }
}

pub(crate) struct SerialBatch {
    pub(crate) lanes: Vec<Lane>,
}

impl SerialBatch {
    pub(crate) fn new(layout: Layout, catalog: &FftCatalog) -> Result<Self, String> {
        let mut lanes = Vec::new();
        lanes
            .try_reserve_exact(WIDTH)
            .map_err(|_| "serial lane allocation")?;
        for _ in 0..WIDTH {
            lanes.push(Lane::new(layout, catalog)?);
        }
        Ok(Self { lanes })
    }

    pub(crate) fn reset(&mut self, base: usize) {
        for (lane, value) in self.lanes.iter_mut().zip(base..) {
            lane.reset(value);
        }
    }

    pub(crate) fn execute(&mut self, operation: Operation) -> Result<(), String> {
        for lane in &mut self.lanes {
            lane.run(operation).map_err(debug)?;
        }
        for lane in &mut self.lanes {
            lane.publish(operation);
        }
        Ok(())
    }
}
