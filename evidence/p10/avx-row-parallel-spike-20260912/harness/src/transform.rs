#![allow(clippy::needless_range_loop)]
use crate::row::dispatches_per_component;
use rustfft::{num_complex::Complex64, Fft, FftPlannerAvx};
use std::{
    sync::Arc,
    time::{Duration, Instant},
};

#[derive(Clone, Copy, Debug, Default)]
pub struct Timing {
    pub pack: Duration,
    pub dispatch: Duration,
    pub fft_phase: Duration,
    pub pure_wait: Duration,
    pub scatter: Duration,
    pub publication: Duration,
    pub blocks: usize,
}

impl Timing {
    pub fn add(&mut self, other: Self) {
        self.pack += other.pack;
        self.dispatch += other.dispatch;
        self.fft_phase += other.fft_phase;
        self.pure_wait += other.pure_wait;
        self.scatter += other.scatter;
        self.publication += other.publication;
        self.blocks += other.blocks;
    }

    pub fn overhead(&self) -> Duration {
        self.pack + self.dispatch + self.pure_wait + self.scatter + self.publication
    }
}

pub struct Plans {
    pub forward: Arc<dyn Fft<f64>>,
    pub inverse: Arc<dyn Fft<f64>>,
    pub scratch_len: usize,
}

impl Plans {
    pub fn new(n: usize) -> Result<Self, String> {
        if !matches!(n, 6 | 384 | 576) {
            return Err("closed prototype length".into());
        }
        let mut planner = FftPlannerAvx::<f64>::new().map_err(|_| "AVX/FMA unavailable")?;
        let forward = planner.plan_fft_forward(n);
        let inverse = planner.plan_fft_inverse(n);
        let scratch_len = forward
            .get_inplace_scratch_len()
            .max(inverse.get_inplace_scratch_len());
        if scratch_len > 4 * n {
            return Err("scratch exceeds audited 4L bound".into());
        }
        Ok(Self {
            forward,
            inverse,
            scratch_len,
        })
    }
}

pub struct Lane {
    pub(crate) n: usize,
    pub(crate) half: usize,
    pub(crate) grid: Vec<Complex64>,
    pub(crate) physical: Vec<f64>,
    pub(crate) spectrum: Vec<Complex64>,
    row: Vec<Complex64>,
    pub(crate) scratch: Vec<Complex64>,
    pub(crate) forward: Arc<dyn Fft<f64>>,
    pub(crate) inverse: Arc<dyn Fft<f64>>,
}

impl Lane {
    pub fn new(n: usize, plans: &Plans) -> Result<Self, String> {
        let half = n / 2 + 1;
        Ok(Self {
            n,
            half,
            grid: zeros(complex_half_len(n)?)?,
            physical: zeros(real_len(n)?)?,
            spectrum: zeros(complex_half_len(n)?)?,
            row: zeros(n)?,
            scratch: zeros(plans.scratch_len.max(4 * n))?,
            forward: plans.forward.clone(),
            inverse: plans.inverse.clone(),
        })
    }

    pub fn reset(&mut self, variant: usize, component: usize) {
        fill_input(&mut self.physical, variant, component);
        self.grid.fill(Complex64::new(0.0, 0.0));
        self.spectrum.fill(Complex64::new(0.0, 0.0));
    }

    pub fn forward_serial(&mut self) -> Result<Timing, String> {
        let started = Instant::now();
        let n = self.n;
        for row_index in 0..n * n {
            for k in 0..n {
                self.row[k] = Complex64::new(self.physical[row_index * n + k], 0.0);
            }
            self.forward
                .process_with_scratch(&mut self.row, &mut self.scratch);
            self.grid[row_index * self.half..(row_index + 1) * self.half]
                .copy_from_slice(&self.row[..self.half]);
        }
        self.transverse_serial(false);
        let scale = real_len(n)? as f64;
        for (output, value) in self.spectrum.iter_mut().zip(&self.grid) {
            *output = *value / scale;
        }
        Ok(Timing {
            fft_phase: started.elapsed(),
            blocks: dispatches_per_component(n),
            ..Timing::default()
        })
    }

    pub fn inverse_serial(&mut self) -> Result<Timing, String> {
        let started = Instant::now();
        self.grid.copy_from_slice(&self.spectrum);
        self.transverse_serial(true);
        let n = self.n;
        for row_index in 0..n * n {
            self.row[..self.half]
                .copy_from_slice(&self.grid[row_index * self.half..(row_index + 1) * self.half]);
            for k in self.half..n {
                self.row[k] = self.row[n - k].conj();
            }
            self.inverse
                .process_with_scratch(&mut self.row, &mut self.scratch);
            for k in 0..n {
                self.physical[row_index * n + k] = self.row[k].re;
            }
        }
        Ok(Timing {
            fft_phase: started.elapsed(),
            blocks: dispatches_per_component(n),
            ..Timing::default()
        })
    }

    fn transverse_serial(&mut self, inverse: bool) {
        let n = self.n;
        for axis in 0..2 {
            let (rows, stride) = if axis == 0 {
                (n, n * self.half)
            } else {
                (n, self.half)
            };
            for outer in 0..rows {
                let row_base = if axis == 0 {
                    outer * self.half
                } else {
                    outer * n * self.half
                };
                for k in 0..self.half {
                    let base = row_base + k;
                    for j in 0..n {
                        self.row[j] = self.grid[base + j * stride];
                    }
                    let plan = if inverse {
                        &self.inverse
                    } else {
                        &self.forward
                    };
                    plan.process_with_scratch(&mut self.row, &mut self.scratch);
                    for j in 0..n {
                        self.grid[base + j * stride] = self.row[j];
                    }
                }
            }
        }
    }

    pub fn snapshot(&self, include_values: bool) -> Snapshot {
        Snapshot::new(&self.spectrum, &self.physical, include_values)
    }

    pub fn direct_error(&self, variant: usize, component: usize) -> Result<f64, String> {
        if self.n != 6 {
            return Err("direct DFT fixture is N6 only".into());
        }
        let mut input = vec![0.0; self.physical.len()];
        fill_input(&mut input, variant, component);
        let direct = direct_dft(self.n, &input);
        Ok(max_scaled(&direct, &self.spectrum))
    }
}

#[derive(Clone)]
pub struct Snapshot {
    pub spectrum_hash: [u8; 32],
    pub physical_hash: [u8; 32],
    pub spectrum: Option<Vec<Complex64>>,
    pub physical: Option<Vec<f64>>,
}

impl Snapshot {
    fn new(spectrum: &[Complex64], physical: &[f64], include: bool) -> Self {
        Self {
            spectrum_hash: hash_complex(spectrum),
            physical_hash: hash_real(physical),
            spectrum: include.then(|| spectrum.to_vec()),
            physical: include.then(|| physical.to_vec()),
        }
    }
}

#[derive(Clone, Copy)]
pub(crate) enum AxisKind {
    RealZ,
    ForwardX,
    ForwardY,
    InverseX,
    InverseY,
    InverseZ,
}

pub fn fill_input(values: &mut [f64], variant: usize, component: usize) {
    for (index, value) in values.iter_mut().enumerate() {
        let raw = ((index.wrapping_mul(31) + variant * 17 + component * 101) % 4093) as f64;
        *value = (raw - 2046.0) / 4096.0;
    }
}

fn hash_complex(values: &[Complex64]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    for v in values {
        h.update(v.re.to_bits().to_le_bytes());
        h.update(v.im.to_bits().to_le_bytes());
    }
    h.finalize().into()
}
fn hash_real(values: &[f64]) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut h = Sha256::new();
    for v in values {
        h.update(v.to_bits().to_le_bytes());
    }
    h.finalize().into()
}

fn direct_dft(n: usize, input: &[f64]) -> Vec<Complex64> {
    let half = n / 2 + 1;
    let scale = (n * n * n) as f64;
    let mut output = vec![Complex64::new(0.0, 0.0); n * n * half];
    for kx in 0..n {
        for ky in 0..n {
            for kz in 0..half {
                let mut sum = Complex64::new(0.0, 0.0);
                for x in 0..n {
                    for y in 0..n {
                        for z in 0..n {
                            let angle =
                                -2.0 * std::f64::consts::PI * ((kx * x + ky * y + kz * z) as f64)
                                    / n as f64;
                            sum += Complex64::new(angle.cos(), angle.sin())
                                * input[(x * n + y) * n + z];
                        }
                    }
                }
                output[(kx * n + ky) * half + kz] = sum / scale;
            }
        }
    }
    output
}

fn max_scaled(a: &[Complex64], b: &[Complex64]) -> f64 {
    a.iter()
        .zip(b)
        .map(|(x, y)| (*x - *y).norm() / x.norm().max(y.norm()).max(1.0))
        .fold(0.0, f64::max)
}

pub(crate) fn checked_product(values: &[usize]) -> Result<usize, String> {
    values.iter().try_fold(1usize, |p, v| {
        p.checked_mul(*v)
            .ok_or_else(|| "reservation overflow".into())
    })
}
pub(crate) fn real_len(n: usize) -> Result<usize, String> {
    checked_product(&[n, n, n])
}
fn complex_half_len(n: usize) -> Result<usize, String> {
    checked_product(&[n, n, n / 2 + 1])
}
pub(crate) fn zeros<T: Clone + Default>(len: usize) -> Result<Vec<T>, String> {
    let mut values = Vec::new();
    values
        .try_reserve_exact(len)
        .map_err(|_| "allocation failed")?;
    values.resize(len, T::default());
    Ok(values)
}
