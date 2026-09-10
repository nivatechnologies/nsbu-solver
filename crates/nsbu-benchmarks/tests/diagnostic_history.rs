//! Reconstruct actual from-rest accepted histories and probe independently between stage clocks.
mod history_support;
use history_support::trajectory_defects;
use nsbu_solver::integrators::method::Method;

#[test]
fn accepted_smooth_histories_have_refining_off_stage_defects() {
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        let mut previous: Option<[f64; 2]> = None;
        for step in [4096_u128, 2048, 1024, 512] {
            let errors = trajectory_defects(method, step);
            println!("accepted-history method={method:?} macro_ticks={step} common_probe_l2={:e} off_stage_max_l2={:e}", errors[0], errors[1]);
            if let Some(previous) = previous {
                let common_order = (previous[0] / errors[0]).log2();
                let sampled_order = (previous[1] / errors[1]).log2();
                assert!(common_order > 2.5);
                assert!(common_order < 3.8);
                assert!(sampled_order > 3.5);
                assert!(sampled_order < 4.5);
            }
            previous = Some(errors);
        }
    }
}
