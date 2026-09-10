//! Independent CM/HO concentrating trajectories and full-band high-precision comparisons.
mod concentrating_support;
mod fixture_support;
use nsbu_solver::{domain::SpectralState, integrators::method::Method};

#[test]
fn concentrating_transactional_trajectory_matches_independent_dft() {
    let cm = concentrating_support::trajectory(Method::CoxMatthews);
    let ho = concentrating_support::trajectory(Method::HochbruckOstermann);
    check_fixture(&cm, include_str!("fixtures/concentrating-n4.tsv"));
    check_fixture(&ho, include_str!("fixtures/concentrating-ho-n4.tsv"));
    compare_methods(&cm, &ho);
}

fn check_fixture(state: &SpectralState, fixture: &str) {
    let output = std::array::from_fn(|axis| state.component(axis).unwrap().to_vec());
    fixture_support::compare(state.plan().domain().layout(), &output, fixture, 5e-13);
}

fn compare_methods(cm: &SpectralState, ho: &SpectralState) {
    let layout = cm.plan().domain().layout();
    let mut squared = 0.0;
    for index in 0..layout.half_len() {
        let weight = layout.weight(layout.position(index).unwrap()).unwrap();
        for component in 0..3 {
            let difference =
                cm.component(component).unwrap()[index] - ho.component(component).unwrap()[index];
            squared += weight * difference.norm_sqr();
        }
    }
    let l2 = squared.sqrt();
    println!("CM/HO diagnostic comparison n=4 fine_dt=1/16384 full_band_l2={l2:e} accepted_pde_windows=0");
    assert!(l2 > 0.0);
    assert!(l2 < 1e-5);
}
