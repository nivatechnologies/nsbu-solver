//! Fixed-size accepted snapshots and an independent double-grid diagnostic path.
use nsbu_benchmarks::smooth::CyclicSine;
use nsbu_solver::{
    diagnostics::{
        conservative::ConservativeWorkspace, hermite::HermiteWeights, residual::ResidualPlan,
    },
    domain::{Domain, SpectralState, TickClock},
    integrators::{forcing::PrescribedForce, method::Method},
    spectral::transfer,
    Complex64,
};

#[path = "../smooth_support/mod.rs"]
mod smooth_support;
use smooth_support::SmoothRun;

type Field = [Vec<Complex64>; 3];
const END: u128 = 1 << 15;

fn empty(domain: Domain) -> Field {
    std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); domain.layout().half_len()])
}
fn slices(field: &Field) -> [&[Complex64]; 3] {
    field.each_ref().map(Vec::as_slice)
}
fn clock(ticks: u128) -> TickClock {
    TickClock::restore(-20, 1 << 20, ticks, (1 << 20) - ticks).unwrap()
}

struct History {
    nodes: [TickClock; 3],
    values: [Field; 3],
    derivatives: [Field; 3],
}
impl History {
    fn new(domain: Domain) -> Self {
        Self {
            nodes: [clock(0); 3],
            values: std::array::from_fn(|_| empty(domain)),
            derivatives: std::array::from_fn(|_| empty(domain)),
        }
    }
    fn capture(&mut self, index: usize, state: &SpectralState, step: u128) {
        self.nodes[index] = state.clock();
        assert_eq!(state.accepted_steps(), state.clock().elapsed() / step);
        for (axis, values) in self.values[index].iter_mut().enumerate() {
            values.copy_from_slice(state.component(axis).unwrap());
        }
    }
    fn reconstruct(&self, probe: TickClock, value: &mut Field, derivative: &mut Field) {
        let weights = HermiteWeights::at(self.nodes, probe).unwrap();
        for axis in 0..3 {
            let samples = std::array::from_fn(|i| {
                if i < 3 {
                    self.values[i][axis].as_slice()
                } else {
                    self.derivatives[i - 3][axis].as_slice()
                }
            });
            weights
                .apply(samples, &mut value[axis], &mut derivative[axis])
                .unwrap();
        }
    }
}

struct Probe {
    source: Domain,
    diagnostic: Domain,
    evaluator: CyclicSine,
    products: ConservativeWorkspace,
    force: Field,
    nonlinear: Field,
    residual: Field,
    pressure: Vec<Complex64>,
    zero: Field,
    calls: usize,
}
impl Probe {
    fn new(source: Domain) -> Self {
        let diagnostic = ConservativeWorkspace::diagnostic_domain(source).unwrap();
        Self {
            source,
            diagnostic,
            evaluator: CyclicSine::new(diagnostic).unwrap(),
            products: ConservativeWorkspace::new(
                source,
                ConservativeWorkspace::reservation(source).unwrap(),
            )
            .unwrap(),
            force: empty(diagnostic),
            nonlinear: empty(diagnostic),
            residual: empty(diagnostic),
            pressure: vec![Complex64::new(0.0, 0.0); diagnostic.layout().half_len()],
            zero: empty(source),
            calls: 0,
        }
    }
    fn nonlinear(&mut self, clock: TickClock, velocity: &Field) {
        assert!(self.calls < 12);
        self.calls += 1;
        let [a, b, c] = &mut self.force;
        let limits = self.evaluator.limits().unwrap();
        let work = self.evaluator.evaluate(clock, limits, [a, b, c]).unwrap();
        assert_eq!(work.scalar_transforms, 0);
        assert!(work.work_units <= limits.work_units);
        let [a, b, c] = &mut self.nonlinear;
        self.products
            .evaluate(
                slices(velocity),
                slices(&self.force),
                [a, b, c],
                &mut self.pressure,
            )
            .unwrap();
    }
    fn derivative(&mut self, clock: TickClock, velocity: &Field, output: &mut Field) {
        self.nonlinear(clock, velocity);
        let [a, b, c] = &mut self.residual;
        ResidualPlan::new(self.source)
            .unwrap()
            .evaluate(
                slices(velocity),
                slices(&self.zero),
                slices(&self.nonlinear),
                [a, b, c],
            )
            .unwrap();
        for (input, output) in self.residual.iter().zip(output) {
            transfer(
                self.diagnostic.layout(),
                self.source.layout(),
                input,
                output,
            )
            .unwrap();
            for value in output {
                *value = -*value;
            }
        }
    }
    fn defect(&mut self, clock: TickClock, velocity: &Field, derivative: &Field) -> f64 {
        self.nonlinear(clock, velocity);
        let [a, b, c] = &mut self.residual;
        ResidualPlan::new(self.source)
            .unwrap()
            .evaluate(
                slices(velocity),
                slices(derivative),
                slices(&self.nonlinear),
                [a, b, c],
            )
            .unwrap()
            .l2
    }
}

pub fn trajectory_defects(method: Method, step: u128) -> [f64; 2] {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let double = ConservativeWorkspace::diagnostic_domain(domain).unwrap();
    // 18 history + 6 reconstruction + 3 zero retained component arrays;
    // 9 vector scratch + 1 pressure double-grid component arrays.
    let diagnostic_bytes = ConservativeWorkspace::reservation(domain).unwrap()
        + 27 * domain.layout().half_len() * 16
        + 10 * double.layout().half_len() * 16;
    let mut run = SmoothRun::new(domain, method, diagnostic_bytes);
    let mut history = History::new(domain);
    let mut probe = Probe::new(domain);
    let mut reconstructed = empty(domain);
    let mut derivative = empty(domain);
    for index in 0..3 {
        let endpoint = END - (2 - index as u128) * step;
        run.advance(endpoint, step);
        history.capture(index, &run.state, step);
        probe.derivative(
            history.nodes[index],
            &history.values[index],
            &mut history.derivatives[index],
        );
    }
    let common_clock = clock(END - 64);
    assert_ne!(common_clock.elapsed() % (step / 4), 0);
    history.reconstruct(common_clock, &mut reconstructed, &mut derivative);
    let common = probe.defect(common_clock, &reconstructed, &derivative);
    let mut maximum = 0.0_f64;
    for numerator in (1..16).step_by(2) {
        let probe_clock = clock(END - 2 * step + numerator * (2 * step / 16));
        assert_ne!(probe_clock.elapsed() % (step / 4), 0);
        history.reconstruct(probe_clock, &mut reconstructed, &mut derivative);
        maximum = maximum.max(probe.defect(probe_clock, &reconstructed, &derivative));
    }
    assert_eq!(probe.calls, 12);
    [common, maximum]
}
