//! Pressure and balance quadrature use accepted smooth states evolved independently from rest.
mod smooth_force_support;
mod smooth_support;
use nsbu_benchmarks::smooth::CyclicSine;
use nsbu_solver::{
    diagnostics::{
        balances::{measure, BalanceSample},
        conservative::ConservativeWorkspace,
        quadrature::simpson,
    },
    domain::{Domain, SpectralState, TickClock},
    integrators::method::Method,
    spectral::transfer,
    Complex64,
};
use smooth_support::SmoothRun;

type Field = [Vec<Complex64>; 3];
const END: u128 = 1 << 15;
const HISTORY: usize = 65;

fn empty(domain: Domain) -> Field {
    std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); domain.layout().half_len()])
}

struct Diagnostic {
    source: Domain,
    double: Domain,
    products: ConservativeWorkspace,
    force: CyclicSine,
    forcing: Field,
    nonlinear: Field,
    padded: Field,
    pressure: Vec<Complex64>,
    calls: usize,
}
impl Diagnostic {
    fn new(source: Domain) -> Self {
        let double = ConservativeWorkspace::diagnostic_domain(source).unwrap();
        Self {
            source,
            double,
            products: ConservativeWorkspace::new(
                source,
                ConservativeWorkspace::reservation(source).unwrap(),
            )
            .unwrap(),
            force: CyclicSine::new(double).unwrap(),
            forcing: empty(double),
            nonlinear: empty(double),
            padded: empty(double),
            pressure: vec![Complex64::new(0.0, 0.0); double.layout().half_len()],
            calls: 0,
        }
    }

    fn sample(&mut self, state: &SpectralState) -> (BalanceSample, f64) {
        assert!(self.calls < HISTORY);
        self.calls += 1;
        let velocity = std::array::from_fn(|axis| state.component(axis).unwrap());
        smooth_force_support::evaluate(&mut self.force, state.clock(), &mut self.forcing);
        let [a, b, c] = &mut self.nonlinear;
        self.products
            .evaluate(
                velocity,
                self.forcing.each_ref().map(Vec::as_slice),
                [a, b, c],
                &mut self.pressure,
            )
            .unwrap();
        for (input, output) in velocity.into_iter().zip(&mut self.padded) {
            transfer(self.source.layout(), self.double.layout(), input, output).unwrap();
        }
        let sample = measure(
            self.double,
            self.padded.each_ref().map(Vec::as_slice),
            self.forcing.each_ref().map(Vec::as_slice),
            self.nonlinear.each_ref().map(Vec::as_slice),
        )
        .unwrap();
        let mut pressure2 = 0.0;
        for (index, value) in self.pressure.iter().enumerate() {
            let position = self.double.layout().position(index).unwrap();
            pressure2 += self.double.layout().weight(position).unwrap() * value.norm_sqr();
        }
        assert!(pressure2.is_finite());
        assert_eq!(self.pressure[0], Complex64::new(0.0, 0.0));
        (sample, pressure2.sqrt())
    }
}

struct Study {
    samples: [BalanceSample; HISTORY],
    count: usize,
    step: u128,
    pressure_peak: f64,
}
impl Study {
    fn run(method: Method, step: u128) -> Self {
        let source = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
        let double = ConservativeWorkspace::diagnostic_domain(source).unwrap();
        let reservation = ConservativeWorkspace::reservation(source).unwrap()
            + 10 * double.layout().half_len() * 16
            + std::mem::size_of::<Self>()
            + std::mem::size_of::<Diagnostic>();
        let mut run = SmoothRun::new(source, method, reservation);
        let mut diagnostic = Diagnostic::new(source);
        let (initial, pressure) = diagnostic.sample(&run.state);
        assert_eq!(initial.energy, 0.0);
        let mut result = Self {
            samples: [initial; HISTORY],
            count: 1,
            step,
            pressure_peak: pressure,
        };
        for elapsed in (step..=END).step_by(step as usize) {
            run.advance(elapsed, step);
            assert_eq!(run.state.accepted_steps(), elapsed / step);
            let (sample, pressure) = diagnostic.sample(&run.state);
            result.samples[result.count] = sample;
            result.count += 1;
            result.pressure_peak = result.pressure_peak.max(pressure);
        }
        assert_eq!(diagnostic.calls, result.count);
        result
    }

    fn balance_defects(&self, stride: usize) -> [f64; 2] {
        let mut integrated = [0.0; 2];
        for start in (0..self.count - 1).step_by(2 * stride) {
            let indexes = [start, start + stride, start + 2 * stride];
            let clocks = indexes.map(|index| {
                let elapsed = index as u128 * self.step;
                TickClock::restore(-20, 1 << 20, elapsed, (1 << 20) - elapsed).unwrap()
            });
            let integral = simpson(clocks, indexes.map(|index| self.samples[index])).unwrap();
            integrated[0] += integral.energy_rhs;
            integrated[1] += integral.enstrophy_rhs;
        }
        [
            self.samples[self.count - 1].energy - self.samples[0].energy - integrated[0],
            self.samples[self.count - 1].enstrophy - self.samples[0].enstrophy - integrated[1],
        ]
    }
}

#[test]
fn accepted_history_pressure_and_balance_refinements_are_recorded_separately() {
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        let mut previous: Option<[f64; 3]> = None;
        for step in [4096, 2048, 1024, 512] {
            let study = Study::run(method, step);
            let defects = study.balance_defects(1);
            println!(
                "method={method:?} step={step} pressure={:.17e} balance={defects:?}",
                study.pressure_peak
            );
            let errors = [study.pressure_peak, defects[0].abs(), defects[1].abs()];
            if let Some(coarse) = previous {
                fourth_order(&coarse, &errors);
            }
            previous = Some(errors);
            if step == 512 {
                quadrature_refinement(&study, method);
            }
        }
    }
}

fn fourth_order(coarse: &[f64], fine: &[f64]) {
    for (coarse, fine) in coarse.iter().zip(fine) {
        let order = (coarse / fine).log2();
        assert!(order > 3.7);
        assert!(order < 4.3);
    }
}

fn quadrature_refinement(study: &Study, method: Method) {
    let mut previous: Option<[f64; 2]> = None;
    for row in include_str!("fixtures/balance-quadrature.tsv").lines() {
        let columns: Vec<&str> = row.split('\t').collect();
        let stride = columns[0].parse().unwrap();
        let defects = study.balance_defects(stride);
        println!("quadrature method={method:?} stride={stride} balance={defects:?}");
        for ((actual, expected), bound) in
            defects.into_iter().zip(&columns[1..]).zip([5e-13, 2e-11])
        {
            let exact_quadrature: f64 = expected.parse().unwrap();
            assert!((actual - exact_quadrature).abs() < bound);
        }
        let errors = defects.map(f64::abs);
        if let Some(coarse) = previous {
            fourth_order(&coarse, &errors);
        }
        previous = Some(errors);
    }
}
