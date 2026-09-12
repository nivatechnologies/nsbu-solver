use nsbu_benchmarks::runtime_force::ForceSettings;
use nsbu_solver::{
    diagnostics::derivatives::{Derivative, DerivativeWorkspace},
    domain::{Domain, Layout, TickClock},
    integrators::forcing::PrescribedForce,
    Complex64,
};

const CAP: usize = 256 * 1024 * 1024;
const ORDERS: [[u8; 3]; 10] = [
    [0, 0, 0], [1, 0, 0], [0, 1, 0], [0, 0, 1], [2, 0, 0],
    [1, 1, 0], [1, 0, 1], [0, 2, 0], [0, 1, 1], [0, 0, 2],
];

fn main() {
    let domain = Domain::new([24; 3], [1.0; 3], 1.0).unwrap();
    let clock = TickClock::restore(-20, 8192, 2048, 6144).unwrap();
    println!("source=31e99a17f97aec2ee18b26c67f8be88a0e931088 retained=N24 clock={clock:?} cap={CAP}");
    for m in [24, 48] {
        inspect(domain, clock, m);
    }
    println!("terminal=complete");
}

fn inspect(domain: Domain, clock: TickClock, m: usize) {
    let settings = ForceSettings { samples: Layout::new([m; 3]).unwrap(), workers: 12 };
    let limits = settings.limits(domain).unwrap();
    let mut force = settings.build(domain, CAP).unwrap();
    let n = domain.layout().half_len();
    let mut field = [vec![Complex64::new(0.0, 0.0); n], vec![Complex64::new(0.0, 0.0); n], vec![Complex64::new(0.0, 0.0); n]];
    let [a,b,c] = &mut field;
    let work = force.evaluate(clock, limits, [a,b,c]).unwrap();
    println!("provider=M{m} limits={limits:?} work={work:?}");
    for axis in 0..3 {
        report_scan(&format!("M{m}/axis{axis}/source"), domain.layout(), &field[axis]);
        let mut sampler = DerivativeWorkspace::new(domain, domain.layout(), CAP).unwrap();
        for orders in ORDERS {
            let derivative = Derivative::new(orders).unwrap();
            let staged = stage(domain, &field[axis], orders);
            report_scan(&format!("M{m}/axis{axis}/d{orders:?}"), domain.layout(), &staged);
            println!("sample=M{m}/axis{axis}/d{orders:?} result={:?}", sampler.sample(&field[axis], derivative).map(|s| (s.values.len(), s.values.iter().fold(0.0_f64, |x,&y| x.max(y.abs())))));
        }
    }
}

fn stage(domain: Domain, input: &[Complex64], orders: [u8; 3]) -> Vec<Complex64> {
    let layout = domain.layout();
    let mut output = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    for (index, &coefficient) in input.iter().enumerate() {
        let position = layout.position(index).unwrap();
        if layout.is_nyquist(position).unwrap() { continue; }
        let mode = layout.mode(position).unwrap();
        let mut value = coefficient;
        for axis in 0..3 {
            let wave = std::f64::consts::TAU * mode[axis] as f64 / domain.lengths()[axis];
            for _ in 0..orders[axis] { value *= Complex64::new(0.0, wave); }
        }
        output[index] = value;
    }
    output
}

fn report_scan(label: &str, layout: Layout, values: &[Complex64]) {
    let [nx, ny, nz] = layout.dimensions();
    let mut nonfinite = 0usize;
    let mut nonzero_nyquist = 0usize;
    let mut worst = (0.0_f64, [0usize;3], Complex64::new(0.0,0.0), Complex64::new(0.0,0.0), 0.0_f64, 0.0_f64);
    for i in 0..nx { for j in 0..ny { for k in 0..=nz/2 {
        let p=[i,j,k]; let a=values[layout.index(p).unwrap()];
        if !a.re.is_finite() || !a.im.is_finite() { nonfinite += 1; }
        if layout.is_nyquist(p).unwrap() && a != Complex64::new(0.0,0.0) { nonzero_nyquist += 1; }
        if k==0 || k==nz/2 {
            let b=values[layout.index([(nx-i)%nx,(ny-j)%ny,k]).unwrap()].conj();
            let scale=a.re.abs().max(a.im.abs()).max(b.re.abs()).max(b.im.abs()).max(1.0);
            let defect=(a-b).re.abs().max((a-b).im.abs());
            let tolerance=64.0*f64::EPSILON*scale;
            let ratio=defect/tolerance;
            if ratio>worst.0 { worst=(ratio,p,a,b,defect,tolerance); }
        }
    }}}
    println!("scan={label} nonfinite={nonfinite} nonzero_nyquist={nonzero_nyquist} worst_ratio={:.17e} mode={:?} a=({:016x},{:016x}) partner_conj=({:016x},{:016x}) defect={:.17e} tolerance={:.17e}",worst.0,layout.mode(worst.1).unwrap(),worst.2.re.to_bits(),worst.2.im.to_bits(),worst.3.re.to_bits(),worst.3.im.to_bits(),worst.4,worst.5);
}
