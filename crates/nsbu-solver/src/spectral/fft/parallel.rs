//! Opt-in bounded intra-transform execution for the AVX backend.
use super::workspace::finite_real;
use super::{BackendPlan, FftBackend, FftPlan, FftWorkspace, AVX_SCRATCH_LANES};
use crate::storage::filled;
use crate::{domain::Layout, Complex64, SolverError};
use rayon::prelude::*;
use rayon::{ThreadPool, ThreadPoolBuilder};
use rustfft::Fft;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};

const STACK_BYTES: usize = 2 * 1024 * 1024;
const CONTROL_ALLOWANCE: usize = 1024 * 1024;
const ALLOCATION_ALLOWANCE: usize = 64;

/// Immutable identity for one opt-in bounded parallel FFT executor.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ParallelFftIdentity {
    /// Transform layout.
    pub layout: Layout,
    /// Fixed arithmetic backend.
    pub backend: FftBackend,
    /// Total Rayon worker count shared by every concurrent caller.
    pub workers: usize,
    /// Complete additional reservation beyond caller-owned scalar lanes.
    pub additional_bytes: usize,
}

struct LineWorkspace {
    input: Vec<Complex64>,
    scratch: Vec<Complex64>,
}

/// Persistent opt-in worker owner shared across scalar or component callers.
///
/// Leaves lock one preallocated line workspace only while running one RustFFT
/// line. Locks are released before Rayon joins. A worker panic or poisoned line
/// permanently terminates the executor; partially written outputs are unspecified.
pub struct ParallelFftExecutor {
    identity: ParallelFftIdentity,
    pool: ThreadPool,
    lines: Vec<Mutex<LineWorkspace>>,
    failed: AtomicBool,
}

impl ParallelFftExecutor {
    /// Conservatively reserve line buffers, fixed stacks, and control storage.
    pub fn additional_reservation(
        layout: Layout,
        backend: FftBackend,
        workers: usize,
    ) -> Result<usize, SolverError> {
        if backend != FftBackend::RustFft6_4_1AvxFma || !(2..=64).contains(&workers) {
            return Err(SolverError::InvalidPayload);
        }
        let maximum = layout
            .dimensions()
            .into_iter()
            .max()
            .ok_or(SolverError::InvalidDomain)?;
        FftPlan::reservation_with_backend(layout, backend)?;
        let elements = maximum
            .checked_mul(1 + AVX_SCRATCH_LANES)
            .ok_or(SolverError::SizeOverflow)?;
        let per_worker = elements
            .checked_mul(size_of::<Complex64>())
            .and_then(|n| n.checked_add(2 * ALLOCATION_ALLOWANCE))
            .and_then(|n| n.checked_add(size_of::<Mutex<LineWorkspace>>()))
            .and_then(|n| n.checked_add(STACK_BYTES))
            .ok_or(SolverError::SizeOverflow)?;
        per_worker
            .checked_mul(workers)
            .and_then(|n| n.checked_add(size_of::<Self>()))
            .and_then(|n| n.checked_add(size_of::<Vec<Mutex<LineWorkspace>>>() + CONTROL_ALLOWANCE))
            .ok_or(SolverError::SizeOverflow)
    }

    /// Construct every worker and its fixed scratch before numerical execution.
    pub fn new(
        layout: Layout,
        backend: FftBackend,
        workers: usize,
        cap: usize,
    ) -> Result<Self, SolverError> {
        let additional = Self::additional_reservation(layout, backend, workers)?;
        if additional > cap {
            return Err(SolverError::ResourceLimit);
        }
        backend.ensure_available()?;
        let maximum = layout
            .dimensions()
            .into_iter()
            .max()
            .ok_or(SolverError::InvalidDomain)?;
        let mut lines = Vec::new();
        lines
            .try_reserve_exact(workers)
            .map_err(|_| SolverError::AllocationFailed)?;
        for _ in 0..workers {
            lines.push(Mutex::new(LineWorkspace {
                input: filled(maximum, Complex64::new(0.0, 0.0))?,
                scratch: filled(
                    AVX_SCRATCH_LANES
                        .checked_mul(maximum)
                        .ok_or(SolverError::SizeOverflow)?,
                    Complex64::new(0.0, 0.0),
                )?,
            }));
        }
        let pool = ThreadPoolBuilder::new()
            .num_threads(workers)
            .stack_size(STACK_BYTES)
            .thread_name(|index| format!("nsbu-fft-{index}"))
            .build()
            .map_err(|_| SolverError::AllocationFailed)?;
        Ok(Self {
            identity: ParallelFftIdentity {
                layout,
                backend,
                workers,
                additional_bytes: additional,
            },
            pool,
            lines,
            failed: AtomicBool::new(false),
        })
    }

    /// Bound layout, backend, worker count, and conservative added storage.
    pub fn identity(&self) -> ParallelFftIdentity {
        self.identity
    }

    /// Whether a worker panic or poisoned scratch permanently terminated this owner.
    pub fn is_terminated(&self) -> bool {
        self.failed.load(Ordering::Acquire)
    }

    /// Forward transform with identical normalization and per-line RustFFT calls.
    pub fn forward(
        &self,
        plan: &FftPlan,
        input: &[f64],
        output: &mut [Complex64],
        work: &mut FftWorkspace,
    ) -> Result<(), SolverError> {
        self.validate(plan, input.len(), output.len(), work)?;
        finite_real(input)?;
        let [nx, ny, nz] = self.identity.layout.dimensions();
        let half = nz / 2 + 1;
        self.execute(|| {
            input
                .par_chunks(nz)
                .zip(work.grid.par_chunks_mut(half))
                .for_each(|(source, target)| {
                    self.line(|line| {
                        for (slot, value) in line.input[..nz].iter_mut().zip(source) {
                            *slot = Complex64::new(*value, 0.0);
                        }
                        process(plan, line, 2, false);
                        target.copy_from_slice(&line.input[..half]);
                    });
                });
        })?;
        plan.transverse_axis_tiled(work, false, 0);
        self.axis1(plan, work, false, nx, ny, half)?;
        let scale = self.identity.layout.real_len() as f64;
        for (value, transformed) in output.iter_mut().zip(&work.grid) {
            *value = transformed / scale;
        }
        crate::spectral::hermitian::finite(output)
    }

    /// Inverse transform with identical Hermitian reconstruction and line arithmetic.
    pub fn inverse(
        &self,
        plan: &FftPlan,
        input: &[Complex64],
        output: &mut [f64],
        work: &mut FftWorkspace,
    ) -> Result<(), SolverError> {
        self.validate(plan, output.len(), input.len(), work)?;
        crate::spectral::hermitian::validate(self.identity.layout, input)?;
        work.grid.copy_from_slice(input);
        let [nx, ny, nz] = self.identity.layout.dimensions();
        let half = nz / 2 + 1;
        plan.transverse_axis_tiled(work, true, 0);
        self.axis1(plan, work, true, nx, ny, half)?;
        self.execute(|| {
            work.grid
                .par_chunks(half)
                .zip(output.par_chunks_mut(nz))
                .for_each(|(source, target)| {
                    self.line(|line| {
                        line.input[..half].copy_from_slice(source);
                        for k in half..nz {
                            line.input[k] = line.input[nz - k].conj();
                        }
                        process(plan, line, 2, true);
                        for (slot, value) in target.iter_mut().zip(&line.input[..nz]) {
                            *slot = value.re;
                        }
                    });
                });
        })?;
        finite_real(output)
    }

    fn validate(
        &self,
        plan: &FftPlan,
        real: usize,
        half: usize,
        work: &FftWorkspace,
    ) -> Result<(), SolverError> {
        if self.is_terminated()
            || !plan.matches_lane(self.identity.layout, self.identity.backend, work)
            || real != self.identity.layout.real_len()
            || half != self.identity.layout.half_len()
        {
            return Err(SolverError::InvalidPayload);
        }
        Ok(())
    }

    fn axis1(
        &self,
        plan: &FftPlan,
        work: &mut FftWorkspace,
        inverse: bool,
        nx: usize,
        ny: usize,
        half: usize,
    ) -> Result<(), SolverError> {
        self.execute(|| {
            work.grid.par_chunks_mut(ny * half).for_each(|slab| {
                self.line(|line| {
                    for k in 0..half {
                        for j in 0..ny {
                            line.input[j] = slab[j * half + k];
                        }
                        process(plan, line, 1, inverse);
                        for j in 0..ny {
                            slab[j * half + k] = line.input[j];
                        }
                    }
                });
            });
        })?;
        debug_assert_eq!(work.grid.len(), nx * ny * half);
        Ok(())
    }

    fn line(&self, use_line: impl FnOnce(&mut LineWorkspace)) {
        if self.is_terminated() {
            return;
        }
        let Some(index) = rayon::current_thread_index() else {
            self.failed.store(true, Ordering::Release);
            return;
        };
        let Ok(mut line) = self.lines[index].lock() else {
            self.failed.store(true, Ordering::Release);
            return;
        };
        use_line(&mut line);
    }

    fn execute(&self, operation: impl FnOnce() + Send) -> Result<(), SolverError> {
        if self.is_terminated() {
            return Err(SolverError::ArithmeticResolutionLimited);
        }
        if catch_unwind(AssertUnwindSafe(|| self.pool.install(operation))).is_err() {
            self.failed.store(true, Ordering::Release);
        }
        if self.is_terminated() {
            Err(SolverError::ArithmeticResolutionLimited)
        } else {
            Ok(())
        }
    }
}

fn process(plan: &FftPlan, line: &mut LineWorkspace, axis: usize, inverse: bool) {
    let BackendPlan::Avx(axes) = &plan.backend else {
        unreachable!("parallel executor validation admits only AVX")
    };
    let fft: &Arc<dyn Fft<f64>> = if inverse {
        &axes[0].inverse[axis]
    } else {
        &axes[0].forward[axis]
    };
    let length = plan.layout.dimensions()[axis];
    let maximum = plan.layout.dimensions().into_iter().max().unwrap();
    fft.process_with_scratch(
        &mut line.input[..length],
        &mut line.scratch[..AVX_SCRATCH_LANES * maximum],
    );
}
