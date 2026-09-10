//! Shared smooth prescribed forcing used by independent step-kernel fixture tests.
use nsbu_solver::{
    domain::{Domain, TickClock},
    integrators::forcing::{ForceLimits, ForceWork, PrescribedForce},
    Complex64, SolverError,
};

pub struct CyclicSource(pub Domain);
impl PrescribedForce for CyclicSource {
    fn limits(&self) -> Option<ForceLimits> {
        Some(ForceLimits {
            storage_bytes: std::mem::size_of::<Self>(),
            work_units: 151,
            scalar_transforms: 0,
            remaining_divisor: 1,
        })
    }
    fn evaluate(
        &mut self,
        time: TickClock,
        _limit: ForceLimits,
        mut output: [&mut [Complex64]; 3],
    ) -> Result<ForceWork, SolverError> {
        let t = time.elapsed() as f64 / 1024.0;
        for (axis, values) in output.iter_mut().enumerate() {
            values.fill(Complex64::new(0.0, 0.0));
            values[0] = Complex64::new([1.0, -2.0, 3.0][axis], 0.0);
        }
        for (axis, mode) in [[0, 1, 0], [0, 0, 1], [1, 0, 0]].into_iter().enumerate() {
            let amplitude = (axis + 1) as f64 * (1.0 + (7.0 * t).sin());
            let value = Complex64::new(0.0, -amplitude / 2.0);
            let (positive, _) = self.0.layout().locate(mode)?;
            output[axis][positive] = value;
            if mode[2] == 0 {
                let (negative, _) = self.0.layout().locate(mode.map(|m| -m))?;
                output[axis][negative] = value.conj();
            }
        }
        Ok(ForceWork {
            work_units: 151,
            scalar_transforms: 0,
        })
    }
}

pub fn payloads(
    plan: nsbu_solver::domain::ResourcePlan,
    clock: TickClock,
) -> (
    nsbu_solver::domain::SpectralState,
    nsbu_solver::integrators::transaction::CandidateState,
) {
    use nsbu_solver::{
        domain::{Epoch, SpectralState},
        integrators::transaction::CandidateState,
    };
    (
        SpectralState::from_rest(plan, clock, Epoch(0)).unwrap(),
        CandidateState::new(plan, clock, Epoch(0)).unwrap(),
    )
}

pub fn plan(domain: Domain, force: usize, diagnostics: usize) -> nsbu_solver::domain::ResourcePlan {
    use nsbu_solver::domain::{Epoch, ExtraStorage, ResourcePlan};
    ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: 0,
            force,
            diagnostics,
            overhead: 4096,
        },
        2 * 1024 * 1024,
        Epoch(0),
    )
    .unwrap()
}

pub fn compare(
    layout: nsbu_solver::domain::Layout,
    full: &[Vec<Complex64>; 3],
    fine: &nsbu_solver::domain::SpectralState,
    fixture: &str,
) {
    for line in fixture.lines() {
        let fields: Vec<&str> = line.split('\t').collect();
        let mode = std::array::from_fn(|i| fields[i + 1].parse::<isize>().unwrap());
        let index = layout.locate(mode).unwrap().0;
        let path = fields[0];
        for axis in 0..3 {
            let expected = Complex64::new(
                fields[4 + axis * 2].parse().unwrap(),
                fields[5 + axis * 2].parse().unwrap(),
            );
            let actual = if fields[0] == "full" {
                full[axis][index]
            } else {
                fine.component(axis).unwrap()[index]
            };
            assert!(
                (actual - expected).norm_sqr() < 1e-28,
                "{path} {mode:?} axis {axis}: {actual} vs {expected}"
            );
        }
    }
}

pub fn arguments(domain: Domain, dt: f64) -> Vec<f64> {
    let layout = domain.layout();
    (0..layout.half_len())
        .map(|index| {
            let position = layout.position(index).unwrap();
            if layout.is_nyquist(position).unwrap() {
                return 0.0;
            }
            let squared = layout
                .mode(position)
                .unwrap()
                .iter()
                .map(|&mode| (std::f64::consts::TAU * mode as f64).powi(2))
                .sum::<f64>();
            -domain.viscosity() * dt * squared
        })
        .collect()
}

pub fn components(state: &nsbu_solver::domain::SpectralState) -> [&[Complex64]; 3] {
    std::array::from_fn(|axis| state.component(axis).unwrap())
}
