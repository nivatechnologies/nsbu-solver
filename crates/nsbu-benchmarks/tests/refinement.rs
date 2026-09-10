//! From-rest nonlinear smooth trajectories with temporal and retained-grid comparisons.
mod smooth_support;
use nsbu_solver::{
    domain::{Domain, SpectralState},
    integrators::method::Method,
    Complex64,
};
use smooth_support::SmoothRun;

fn trajectory(n: usize, divisor: usize, method: Method) -> SpectralState {
    let domain = Domain::new([n; 3], [1.0; 3], 1.0).unwrap();
    let mut run = SmoothRun::new(domain, method, 0);
    run.advance(1 << 15, (1 << 20) / divisor as u128);
    run.state
}

fn errors(state: &SpectralState) -> [f64; 2] {
    let layout = state.plan().domain().layout();
    let amplitudes = [13.0_f64, 17.0, 19.0].map(|frequency| (frequency / 32.0).sin());
    let mut squared = [0.0; 2];
    for index in 0..layout.half_len() {
        let (mode, weight) = mode_weight(layout, index);
        let frequency2 =
            std::f64::consts::TAU.powi(2) * mode.iter().map(|m| (m * m) as f64).sum::<f64>();
        for (axis, amplitude) in amplitudes.into_iter().enumerate() {
            let mut sine_mode = [0; 3];
            sine_mode[(axis + 1) % 3] = mode[(axis + 1) % 3];
            let expected = if mode == sine_mode && sine_mode[(axis + 1) % 3].abs() == 1 {
                Complex64::new(0.0, -amplitude * mode[(axis + 1) % 3] as f64 / 2.0)
            } else {
                Complex64::new(0.0, 0.0)
            };
            let difference = state.component(axis).unwrap()[index] - expected;
            let error = weight * difference.norm_sqr();
            squared[0] += error;
            squared[1] += (1.0 + frequency2) * error;
        }
    }
    squared.map(f64::sqrt)
}

#[test]
fn cm_smooth_nonlinear_temporal_order_from_rest() {
    temporal_order(Method::CoxMatthews);
}

#[test]
fn ho_smooth_nonlinear_temporal_order_from_rest() {
    temporal_order(Method::HochbruckOstermann);
}

fn temporal_order(method: Method) {
    let mut previous = None;
    for divisor in [128, 256, 512, 1024] {
        let error = errors(&trajectory(4, divisor, method));
        println!(
            "smooth-time method={method:?} macro_divisor={divisor} l2={:e} h1={:e}",
            error[0], error[1]
        );
        if let Some(coarse) = previous {
            let order = f64::log2(coarse / error[1]);
            assert!(order > 3.5);
            assert!(order < 4.5);
        }
        previous = Some(error[1]);
    }
}

#[test]
fn smooth_full_band_spatial_family_tracks_the_same_exact_flow() {
    let mut previous = None;
    let mut previous_gap = None;
    for n in [4, 8, 12] {
        let state = trajectory(n, 512, Method::CoxMatthews);
        let error = errors(&state);
        println!(
            "smooth-space n={n} macro_divisor=512 l2={:e} h1={:e}",
            error[0], error[1]
        );
        assert!(error[0] < 1e-8);
        assert!(error[1] < 1e-7);
        if let Some(coarse) = previous {
            let gap = spatial_difference(&coarse, &state);
            println!("smooth-space full-fine-band-gap n={n} l2={gap:e}");
            if let Some(older) = previous_gap {
                assert!(gap < older * 1e-4);
            }
            previous_gap = Some(gap);
        }
        previous = Some(state);
    }
}

fn spatial_difference(coarse: &SpectralState, fine: &SpectralState) -> f64 {
    let layout = fine.plan().domain().layout();
    let coarse_layout = coarse.plan().domain().layout();
    let mut square = 0.0;
    for index in 0..layout.half_len() {
        let (mode, weight) = mode_weight(layout, index);
        for axis in 0..3 {
            let earlier = coarse_layout
                .locate(mode)
                .map_or(Complex64::new(0.0, 0.0), |(i, _)| {
                    coarse.component(axis).unwrap()[i]
                });
            let difference = fine.component(axis).unwrap()[index] - earlier;
            square += weight * difference.norm_sqr();
        }
    }
    square.sqrt()
}

fn mode_weight(layout: nsbu_solver::domain::Layout, index: usize) -> ([isize; 3], f64) {
    let position = layout.position(index).unwrap();
    (
        layout.mode(position).unwrap(),
        layout.weight(position).unwrap(),
    )
}
