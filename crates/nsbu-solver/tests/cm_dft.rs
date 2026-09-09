//! Production FFT/CM steps compared with independent 120-digit direct-DFT evolution from rest.
use nsbu_solver::domain::{Domain, Epoch, ExtraStorage, ResourcePlan, SpectralState, TickClock};
use nsbu_solver::integrators::{
    attempt::AttemptWorkspace,
    coefficients::CmCoefficients,
    forcing::{ForceLimits, ForceWork, PrescribedForce},
    indicator::Tolerances,
    kernel::{CmWorkspace, RightHandSide},
    rhs::SpectralRhs,
    time::binary_duration,
    transaction::{commit_candidate, CandidateState},
};
use nsbu_solver::{Complex64, SolverError};

struct CyclicSource(Domain);
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

#[test]
fn full_and_two_half_steps_match_independent_full_band_fixtures() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let layout = domain.layout();
    let source_bytes =
        SpectralRhs::<CyclicSource>::reservation(domain, CyclicSource(domain).limits().unwrap())
            .unwrap();
    let plan = ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: 0,
            force: source_bytes,
            diagnostics: AttemptWorkspace::reservation(domain).unwrap(),
            overhead: 4096,
        },
        2 * 1024 * 1024,
        Epoch(0),
    )
    .unwrap();
    let clock = TickClock::from_rest(-10, 1024).unwrap();
    let mut state = SpectralState::from_rest(plan, clock, Epoch(0)).unwrap();
    let mut candidate = CandidateState::new(plan, clock, Epoch(0)).unwrap();
    let mut rhs = SpectralRhs::new(domain, CyclicSource(domain), 0.3, source_bytes).unwrap();
    let mut kernel = CmWorkspace::new(layout.half_len(), plan.total()).unwrap();
    let mut coefficients = vec![CmCoefficients::new(0.0).unwrap(); layout.half_len()];
    let dt = binary_duration(8, clock.exponent()).unwrap();
    for (i, c) in coefficients.iter_mut().enumerate() {
        let position = layout.position(i).unwrap();
        if layout.is_nyquist(position).unwrap() {
            continue;
        }
        let squared = layout
            .mode(position)
            .unwrap()
            .iter()
            .map(|&m| (std::f64::consts::TAU * m as f64).powi(2))
            .sum::<f64>();
        *c = CmCoefficients::new(-dt * squared).unwrap();
    }
    let mut full =
        std::array::from_fn::<_, 3, _>(|_| vec![Complex64::new(0.0, 0.0); layout.half_len()]);
    let stages = clock.stages(8).unwrap();
    let [a, b, c] = &mut full;
    rhs.begin_attempt(clock, 8).unwrap();
    kernel
        .step(
            [
                state.component(0).unwrap(),
                state.component(1).unwrap(),
                state.component(2).unwrap(),
            ],
            [stages[0], stages[2], stages[4]],
            dt,
            &coefficients,
            &mut rhs,
            [a, b, c],
        )
        .unwrap();
    let mut work = AttemptWorkspace::new(plan).unwrap();
    let result = work
        .try_advance(
            &state,
            &mut candidate,
            8,
            Tolerances {
                absolute: [1.0; 2],
                relative: [0.0; 2],
            },
            &mut rhs,
        )
        .unwrap();
    commit_candidate(plan, &mut state, &mut candidate, result.accepted.unwrap()).unwrap();
    compare_fixture(layout, &full, &state);
}

fn compare_fixture(
    layout: nsbu_solver::domain::Layout,
    full: &[Vec<Complex64>; 3],
    fine: &SpectralState,
) {
    for line in include_str!("fixtures/cm-step.tsv").lines() {
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
