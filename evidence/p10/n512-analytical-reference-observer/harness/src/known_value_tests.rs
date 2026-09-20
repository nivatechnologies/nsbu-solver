#![allow(clippy::too_many_lines)]
//! Independent known-value fixtures: hand-derived analytic expectations for
//! derivative, curl, Hessian, gauge and peak witnesses, computed from closed
//! forms (single Fourier modes) rather than from the code under test.
use crate::cache;
use crate::quantity::{self, FieldPeak, PeakSource, SampleMaximum, axis_distance, maximum};
use nsbu_benchmarks::fields::reference::ReferenceEvaluation;
use nsbu_solver::{
    Complex64,
    diagnostics::{
        derivatives::{Derivative, DerivativeWorkspace},
        physical::PhysicalQuantity,
    },
    domain::{Domain, Layout, TickClock},
};

const TAU: f64 = std::f64::consts::TAU;
const TOL: f64 = 1e-9;

/// Source grid 4^3 (only |mode| < 2 admitted); sample lattice 8^3, so every
/// expectation below is a closed form evaluated at `point(samples, index)`.
fn source() -> Domain {
    Domain::new([4; 3], [1.0; 3], 1.0).expect("fixture domain")
}

fn samples() -> Layout {
    Layout::new([8; 3]).expect("fixture samples")
}

fn clock() -> TickClock {
    TickClock::restore(-20, 8_192, 1_024, 7_168).expect("fixture clock")
}

/// The analytic field for component 0:
/// `u0 = cos(tau x) + 0.5 cos(tau y) + 0.5 sin(tau y)
///      + 0.5 cos(tau z) + 0.25 sin(tau z)`.
fn mode_field() -> [Vec<Complex64>; 3] {
    let layout = source().layout();
    let zero = Complex64::new(0.0, 0.0);
    let mut component = vec![zero; layout.half_len()];
    store(&mut component, [1, 0, 0], Complex64::new(0.5, 0.0));
    store(&mut component, [0, 1, 0], Complex64::new(0.25, -0.25));
    store(&mut component, [0, 0, 1], Complex64::new(0.25, -0.125));
    [component, vec![zero; layout.half_len()], vec![zero; layout.half_len()]]
}

/// Component 1: `u1 = 0.25 cos(tau (x+y)) - 0.125 sin(tau (x+y))`.
fn coupled_component() -> Vec<Complex64> {
    let layout = source().layout();
    let mut component = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    store(&mut component, [1, 1, 0], Complex64::new(0.125, 0.0625));
    component
}

/// Store one complex mode with its Hermitian partner. Slots found through a
/// conjugated lookup hold the conjugate of the physical mode coefficient.
fn store(component: &mut [Complex64], mode: [isize; 3], amplitude: Complex64) {
    let layout = source().layout();
    let (index, conjugate) = layout.locate(mode).expect("band mode");
    component[index] = if conjugate { amplitude.conj() } else { amplitude };
    let negative = [-mode[0], -mode[1], -mode[2]];
    let (index, conjugate) = layout.locate(negative).expect("band partner");
    component[index] = if conjugate { amplitude } else { amplitude.conj() };
}

/// Closed-form velocity, gradient, Hessian and curl; independent of the
/// library samplers.
type JetWitness = ([f64; 3], [[f64; 3]; 3], [[[f64; 3]; 3]; 3], [f64; 3]);

fn closed_form(point: [f64; 3]) -> JetWitness {
    let [x, y, z] = point;
    let (sx, cx) = (TAU * x).sin_cos();
    let (sy, cy) = (TAU * y).sin_cos();
    let (sz, cz) = (TAU * z).sin_cos();
    let y_part = 0.5 * cy + 0.5 * sy;
    let z_part = 0.5 * cz + 0.25 * sz;
    let u0 = cx + y_part + z_part;
    let theta = TAU * (x + y);
    let (st, ct) = theta.sin_cos();
    let u1 = 0.25 * ct - 0.125 * st;
    let dx_u0 = -TAU * sx;
    let dy_u0 = TAU * (0.5 * cy - 0.5 * sy);
    let dz_u0 = TAU * (0.25 * cz - 0.5 * sz);
    let d_u1 = -TAU * (0.25 * st + 0.125 * ct);
    let velocity = [u0, u1, 0.0];
    let mut gradient = [[0.0; 3]; 3];
    gradient[0] = [dx_u0, dy_u0, dz_u0];
    gradient[1] = [d_u1, d_u1, 0.0];
    let mut hessian = [[[0.0; 3]; 3]; 3];
    hessian[0][0][0] = -TAU * TAU * cx;
    hessian[0][1][1] = -TAU * TAU * y_part;
    hessian[0][2][2] = -TAU * TAU * z_part;
    for (a, b) in [(0, 0), (0, 1), (1, 0), (1, 1)] {
        hessian[1][a][b] = -TAU * TAU * u1;
    }
    let curl = [0.0, dz_u0, d_u1 - dy_u0];
    (velocity, gradient, hessian, curl)
}

fn sample(
    orders: [u8; 3],
    spectrum: &[Complex64],
    workspace: &mut DerivativeWorkspace,
) -> Vec<f64> {
    workspace
        .sample(spectrum, Derivative::new(orders).expect("derivative orders"))
        .expect("sample")
        .values
        .to_vec()
}

#[test]
fn first_derivatives_match_closed_forms() {
    let mut workspace = DerivativeWorkspace::new(source(), samples(), 1 << 26).expect("sampler");
    let field = mode_field();
    for (axis, name) in [(0, "x"), (1, "y"), (2, "z")] {
        let mut orders = [0; 3];
        orders[axis] = 1;
        let values = sample(orders, &field[0], &mut workspace);
        for (index, &value) in values.iter().enumerate() {
            let (_, gradient, ..) = closed_form(cache::point(samples(), index));
            let expected = gradient[0][axis];
            assert!(
                (value - expected).abs() <= TOL * expected.abs().max(1.0),
                "d/d{name} point {index}: {value} != {expected}"
            );
        }
    }
}

#[test]
fn second_derivatives_and_mixed_hessians_match_closed_forms() {
    let mut workspace = DerivativeWorkspace::new(source(), samples(), 1 << 26).expect("sampler");
    let field = mode_field();
    let coupled = coupled_component();
    let cases: [(usize, [u8; 3], [usize; 2]); 8] = [
        (0, [2, 0, 0], [0, 0]),
        (0, [0, 2, 0], [1, 1]),
        (0, [0, 0, 2], [2, 2]),
        (0, [1, 1, 0], [0, 1]),
        (0, [1, 0, 1], [0, 2]),
        (0, [0, 1, 1], [1, 2]),
        (1, [2, 0, 0], [0, 0]),
        (1, [1, 1, 0], [0, 1]),
    ];
    for (row, orders, pair) in cases {
        let spectrum = if row == 0 { &field[0] } else { &coupled };
        let values = sample(orders, spectrum, &mut workspace);
        for (index, &value) in values.iter().enumerate() {
            let (_, _, hessian, _) = closed_form(cache::point(samples(), index));
            let expected = hessian[row][pair[0]][pair[1]];
            assert!(
                (value - expected).abs() <= 1e-7 * expected.abs().max(1.0),
                "row {row} orders {orders:?} point {index}: {value} != {expected}"
            );
        }
    }
}

fn reference_lattice() -> Vec<ReferenceEvaluation> {
    (0..samples().real_len())
        .map(|index| {
            let (velocity, gradient, hessian, vorticity) =
                closed_form(cache::point(samples(), index));
            ReferenceEvaluation {
                velocity,
                gradient,
                hessian,
                vorticity,
                pressure_raw: 0.0,
                pressure_gradient: [0.0; 3],
                root: None,
            }
        })
        .collect()
}

#[test]
fn curl_witness_matches_closed_form_curl() {
    let mut workspace = DerivativeWorkspace::new(source(), samples(), 1 << 26).expect("sampler");
    let mut field = mode_field();
    field[1] = coupled_component();
    let spectra = [&field[0][..], &field[1][..], &field[2][..]];
    let reference = reference_lattice();
    let side = quantity::ReferenceSide::Velocity(&reference);
    let points = samples().real_len();
    let mut actual = vec![0.0; points];
    let mut scratch = vec![0.0; points];
    let mut errors = vec![0.0; points];
    let mut references = vec![0.0; points];
    let finding = quantity::measure::<3>(
        "vorticity",
        PhysicalQuantity::Vorticity,
        &mut workspace,
        &spectra,
        &side,
        samples(),
        clock(),
        1e-12,
        32,
        &mut actual,
        &mut scratch,
        &mut errors,
        &mut references,
    )
    .expect("curl measurement");
    assert_eq!(finding.transforms, 6, "curl charges two samples per component");
    assert!(
        finding.global.rms_error <= 1e-8 && finding.global.peak_error <= 1e-8,
        "closed-form curl mismatch: {:?}",
        finding.global
    );
    assert!(
        finding.error_peak.maximum.value <= 1e-8
            && finding.error_peak.source == PeakSource::Error
            && finding.error_peak.field == "vorticity"
            && finding.error_peak.identity() == "vorticity:error",
        "typed error peak witness: {:?}",
        finding.error_peak
    );
    assert!(finding.relative_peak.maximum.value <= 1e-7);
    assert!(finding.peak_height_error.abs() <= 1e-6);
}

#[test]
fn vector_gradient_and_hessian_measurements_agree_with_closed_forms() {
    let mut workspace = DerivativeWorkspace::new(source(), samples(), 1 << 26).expect("sampler");
    let mut field = mode_field();
    field[1] = coupled_component();
    let spectra = [&field[0][..], &field[1][..], &field[2][..]];
    let reference = reference_lattice();
    let side = quantity::ReferenceSide::Velocity(&reference);
    let points = samples().real_len();
    for (name, kind, transforms) in [
        ("velocity", PhysicalQuantity::Vector, 3),
        ("gradient", PhysicalQuantity::Gradient, 9),
        ("hessian", PhysicalQuantity::Hessian, 27),
    ] {
        let mut actual = vec![0.0; points];
        let mut scratch = vec![0.0; points];
        let mut errors = vec![0.0; points];
        let mut references = vec![0.0; points];
        let finding = match kind {
            PhysicalQuantity::Gradient => quantity::measure::<9>(
                name, kind, &mut workspace, &spectra, &side, samples(), clock(), 1e-12, 32,
                &mut actual, &mut scratch, &mut errors, &mut references,
            ),
            PhysicalQuantity::Hessian => quantity::measure::<27>(
                name, kind, &mut workspace, &spectra, &side, samples(), clock(), 1e-12, 32,
                &mut actual, &mut scratch, &mut errors, &mut references,
            ),
            _ => quantity::measure::<3>(
                name, kind, &mut workspace, &spectra, &side, samples(), clock(), 1e-12, 32,
                &mut actual, &mut scratch, &mut errors, &mut references,
            ),
        }
        .expect("measurement");
        assert_eq!(finding.transforms, transforms, "{name} transform count");
        assert!(
            finding.global.rms_error <= 1e-7 && finding.global.peak_error <= 1e-7,
            "{name} closed-form mismatch: {:?}",
            finding.global
        );
        assert_eq!(finding.actual_peak.source, PeakSource::Actual);
        assert_eq!(finding.reference_peak.source, PeakSource::Reference);
        assert_eq!(finding.actual_peak.identity(), format!("{name}:actual"));
    }
}

#[test]
fn gauge_fixture_has_exact_hand_computed_peaks_and_offset_cancellation() {
    let mut workspace = DerivativeWorkspace::new(source(), samples(), 1 << 26).expect("sampler");
    let points = samples().real_len();
    let rows: Vec<[f64; 4]> = (0..points)
        .map(|index| {
            [
                994.0 + (index * 37) as f64 % 13.0,
                index as f64 % 5.0 - 2.0,
                0.5,
                -0.25,
            ]
        })
        .collect();
    let ordered_mean = cache::lattice_mean(&rows, 0).expect("gauge mean");
    let zero_spectrum = vec![Complex64::new(0.0, 0.0); source().layout().half_len()];
    let spectra = [&zero_spectrum[..]];
    let side = quantity::ReferenceSide::Pressure {
        rows: &rows,
        mean: ordered_mean,
    };
    let mut actual = vec![0.0; points];
    let mut scratch = vec![0.0; points];
    let mut errors = vec![0.0; points];
    let mut references = vec![0.0; points];
    let finding = quantity::measure::<1>(
        "pressure",
        PhysicalQuantity::Scalar,
        &mut workspace,
        &spectra,
        &side,
        samples(),
        clock(),
        1e-12,
        32,
        &mut actual,
        &mut scratch,
        &mut errors,
        &mut references,
    )
    .expect("gauged pressure measurement");
    assert_eq!(finding.transforms, 1);
    assert_eq!(finding.actual_peak.maximum.value, 0.0);
    assert_eq!(finding.actual_peak.maximum.linear, 0);
    let expected_peak = rows
        .iter()
        .enumerate()
        .map(|(index, row)| (index, (row[0] - ordered_mean).abs()))
        .fold(
            (0_usize, 0.0_f64),
            |best, (index, value)| {
                if value > best.1 { (index, value) } else { best }
            },
        );
    assert_eq!(finding.error_peak.maximum.linear, expected_peak.0);
    assert_eq!(finding.error_peak.maximum.value, expected_peak.1);
    assert_eq!(finding.reference_peak.maximum.linear, expected_peak.0);
    assert_eq!(finding.relative_peak.maximum.value, 1.0, "relative peak is exactly one");
    assert_eq!(finding.peak_height_error, -expected_peak.1);
    assert_eq!(
        finding.error_peak.maximum.index,
        expected_peak.0
            .to_witness_index(samples())
    );
}

/// Witness de-composition in x-major, z-fastest order (mirror of the reviewed
/// contract, written independently here).
trait ToWitness {
    fn to_witness_index(self, layout: Layout) -> [usize; 3];
}

impl ToWitness for usize {
    fn to_witness_index(self, layout: Layout) -> [usize; 3] {
        let [_, ny, nz] = layout.dimensions();
        [self / (ny * nz), (self / nz) % ny, self % nz]
    }
}

#[test]
fn typed_field_peak_identity_is_known() {
    let peak = FieldPeak::new(
        "pressure",
        PeakSource::RelativeError,
        SampleMaximum {
            linear: 3,
            index: [0, 0, 3],
            value: 2.5,
        },
    );
    assert_eq!(peak.identity(), "pressure:relative-error");
    assert_eq!(PeakSource::Actual.as_str(), "actual");
    assert_eq!(PeakSource::Reference.as_str(), "reference");
    assert_eq!(PeakSource::Error.as_str(), "error");
    assert_eq!(PeakSource::RelativeError.as_str(), "relative-error");
    assert_eq!(axis_distance(10, 8, 1), 3);
    assert_eq!(axis_distance(10, 5, 0), 5);
    assert_eq!(axis_distance(10, 0, 6), 4);
}

#[test]
fn witness_indices_use_x_major_z_fast_order_on_rectangular_lattices() {
    let layout = Layout::new([4, 6, 8]).expect("rectangular layout");
    let mut values = vec![0.0; 4 * 6 * 8];
    values[27] = 7.0;
    let peak = maximum(&values, 4 * 6 * 8, layout).expect("maximum");
    assert_eq!(peak.linear, 27);
    assert_eq!(peak.index, [0, 3, 3]);
    assert_eq!(peak.value, 7.0);
    values[0] = 7.0;
    let tied = maximum(&values, 4 * 6 * 8, layout).expect("tie keeps first");
    assert_eq!(tied.linear, 0, "first maximizer in x-major z-fastest order");
}
