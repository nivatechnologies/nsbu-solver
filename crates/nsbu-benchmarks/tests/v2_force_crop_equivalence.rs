//! Exact-word prerequisite for a future shared original-force owner.
use nsbu_benchmarks::runtime_force::ForceSettings;
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
    integrators::forcing::{ForceWork, PrescribedForce},
    spectral::transfer,
    Complex64, SolverError,
};

type Field = [Vec<Complex64>; 3];

fn domain(n: usize) -> Domain {
    Domain::new([n; 3], [1.0; 3], 1.0).unwrap()
}

fn field(layout: Layout) -> Field {
    std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); layout.half_len()])
}

fn evaluate(retained: Domain, settings: ForceSettings, clock: TickClock) -> (Field, ForceWork) {
    let limits = settings.limits(retained).unwrap();
    let mut force = settings.build(retained, limits.storage_bytes).unwrap();
    let mut values = field(retained.layout());
    let work = force
        .evaluate(clock, limits, values.each_mut().map(Vec::as_mut_slice))
        .unwrap();
    (values, work)
}

fn independent_crop(source: Layout, target: Layout, input: &[Complex64]) -> Vec<Complex64> {
    assert_eq!(input.len(), source.half_len());
    let [nx, ny, _] = target.dimensions();
    let mut output = vec![Complex64::new(0.0, 0.0); target.half_len()];
    for i in 0..nx {
        for j in 0..ny {
            crop_row(source, target, input, &mut output, [i, j]);
        }
    }
    output
}

fn crop_row(
    source: Layout,
    target: Layout,
    input: &[Complex64],
    output: &mut [Complex64],
    row: [usize; 2],
) {
    let [nx, ny, nz] = target.dimensions();
    if row[0] == nx / 2 || row[1] == ny / 2 {
        return;
    }
    for k in 0..nz / 2 {
        let mode = [signed(row[0], nx), signed(row[1], ny), k as isize];
        let (source_index, _) = source.locate(mode).unwrap();
        let target_index = (row[0] * ny + row[1]) * (nz / 2 + 1) + k;
        output[target_index] = input[source_index];
    }
}

fn signed(index: usize, length: usize) -> isize {
    if index <= length / 2 {
        index as isize
    } else {
        index as isize - length as isize
    }
}

fn words(values: &[Complex64]) -> Vec<(u64, u64)> {
    values
        .iter()
        .map(|value| (value.re.to_bits(), value.im.to_bits()))
        .collect()
}

fn assert_strict_nyquist_zero(layout: Layout, values: &[Complex64]) {
    let [nx, ny, _] = layout.dimensions();
    for i in 0..nx {
        for j in 0..ny {
            assert_nyquist_row(layout, values, [i, j]);
        }
    }
}

fn assert_nyquist_row(layout: Layout, values: &[Complex64], row: [usize; 2]) {
    let [nx, ny, nz] = layout.dimensions();
    let half = nz / 2 + 1;
    for k in 0..half {
        if is_nyquist([nx, ny, nz], row, k) {
            let value = values[(row[0] * ny + row[1]) * half + k];
            assert_eq!(value.re.to_bits(), 0.0_f64.to_bits());
            assert_eq!(value.im.to_bits(), 0.0_f64.to_bits());
        }
    }
}

fn is_nyquist(shape: [usize; 3], row: [usize; 2], k: usize) -> bool {
    row[0] == shape[0] / 2 || row[1] == shape[1] / 2 || k == shape[2] / 2
}

fn compare_component(small: Domain, large: Domain, direct: &[Complex64], common: &[Complex64]) {
    let expected = independent_crop(large.layout(), small.layout(), common);
    assert_eq!(words(direct), words(&expected));
    assert_eq!(direct[0].re.to_bits(), expected[0].re.to_bits());
    assert_eq!(direct[0].im.to_bits(), expected[0].im.to_bits());
    assert_strict_nyquist_zero(small.layout(), direct);
}

fn has_nonzero(values: &[Complex64]) -> bool {
    values
        .iter()
        .any(|value| value.re != 0.0 || value.im != 0.0)
}

fn assert_activity(clock: TickClock, observed_nonzero: bool) {
    if clock.elapsed() == 0 {
        assert!(!observed_nonzero);
    } else {
        assert!(observed_nonzero);
    }
}

fn assert_force_crop(case: (usize, usize, usize), workers: usize, clock: TickClock) {
    let (small_n, large_n, samples_n) = case;
    let settings = ForceSettings {
        samples: Layout::new([samples_n; 3]).unwrap(),
        workers,
    };
    let small = domain(small_n);
    let large = domain(large_n);
    let (direct, direct_work) = evaluate(small, settings, clock);
    let (common, common_work) = evaluate(large, settings, clock);
    assert_eq!(direct_work.work_units, common_work.work_units);
    assert_eq!(direct_work.scalar_transforms, common_work.scalar_transforms);

    let mut observed_nonzero = false;
    for component in 0..3 {
        compare_component(small, large, &direct[component], &common[component]);
        observed_nonzero |= has_nonzero(&direct[component]);
    }
    assert_activity(clock, observed_nonzero);
}

#[test]
fn direct_force_equals_common_domain_crop_at_every_word() {
    let clocks = [
        TickClock::from_rest(-20, 8192).unwrap(),
        TickClock::restore(-20, 8192, 2047, 6145).unwrap(),
    ];
    for workers in [0, 2] {
        for clock in clocks {
            // Integration-scale and independently doubled observer-scale configurations.
            assert_force_crop((4, 8, 16), workers, clock);
            assert_force_crop((8, 16, 32), workers, clock);
        }
    }
}

#[test]
fn independent_crop_checks_signed_modes_dc_and_nyquist() {
    let source = Layout::new([8; 3]).unwrap();
    let target = Layout::new([4; 3]).unwrap();
    let input: Vec<_> = (0..source.half_len())
        .map(|index| Complex64::new(index as f64 + 1.0, -(index as f64) - 2.0))
        .collect();
    let expected = independent_crop(source, target, &input);
    let mut actual = vec![Complex64::new(7.0, 9.0); target.half_len()];
    transfer(source, target, &input, &mut actual).unwrap();
    assert_eq!(words(&actual), words(&expected));

    for mode in [[0, 0, 0], [1, 0, 0], [-1, 0, 0], [0, -1, 1]] {
        let (small_index, _) = target.locate(mode).unwrap();
        let (large_index, _) = source.locate(mode).unwrap();
        assert_eq!(actual[small_index], input[large_index]);
    }
    assert_strict_nyquist_zero(target, &actual);
}

#[test]
fn exact_v2_case_refuses_nonunit_physical_domains() {
    let samples = Layout::new([16; 3]).unwrap();
    for invalid in [
        Domain::new([8; 3], [2.0, 1.0, 1.0], 1.0).unwrap(),
        Domain::new([8; 3], [1.0; 3], 2.0).unwrap(),
    ] {
        assert_eq!(
            ForceSettings {
                samples,
                workers: 0,
            }
            .limits(invalid),
            Err(SolverError::InvalidDomain)
        );
    }
}
