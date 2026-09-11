//! Bounded analytical velocity/derivative tracking of six actual exact-v2 trajectories.
use nsbu_benchmarks::{
    runtime_force::ForceSettings,
    v2_experiment::{
        reference::{ReferenceTrackingPlan, ReferenceTrackingWorkspace},
        FamilyPlan, FamilySettings, V2Family,
    },
    CASE_SHA256,
};
use nsbu_solver::{
    domain::{Layout, TickClock},
    integrators::indicator::Tolerances,
    verification::times::TestedTimes,
};

fn main() {
    let clocks =
        [0, 64, 128].map(|elapsed| TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap());
    let family_settings = FamilySettings {
        grids: [4, 8, 12],
        steps: [64, 32, 16],
        force: ForceSettings {
            samples: Layout::new([12; 3]).unwrap(),
            workers: 0,
        },
        endpoint: 128,
        tolerances: Tolerances {
            absolute: [1e-5, 1e-4],
            relative: [0.0; 2],
        },
        advective_limit: 0.3,
    };
    let cap = 256 * 1024 * 1024;
    let family_plan = FamilyPlan::new(
        family_settings,
        TestedTimes::new(&clocks, clocks.len()).unwrap(),
        cap,
    )
    .unwrap();
    let tracking_plan = ReferenceTrackingPlan::new(
        family_plan,
        Layout::new([12; 3]).unwrap(),
        [1e-8, 1e-7, 1e-6, 1e-7],
        clocks.len(),
        cap,
    )
    .unwrap();
    println!("case=similarity-mms-v2 sha256={CASE_SHA256}");
    println!(
        "family_identity={:02x?} reference_tracking_preflight={:?}",
        family_plan.identity(),
        tracking_plan.bounds()
    );
    println!("family_grids=[4,8,12] fixed_force_grid=12 steps=[64,32,16] samples=12 quantum=2^-20 endpoint=128");
    let mut family = V2Family::new(family_plan).unwrap();
    let mut tracking = ReferenceTrackingWorkspace::new(tracking_plan).unwrap();
    while let Some(family_sample) = family.advance().unwrap() {
        let sample = tracking.measure(&family).unwrap();
        assert_eq!(sample.clock(), family_sample.clock());
        for branch in sample.branches() {
            println!(
                "elapsed={} branch={} tracking_RMS={:?} tracking_peaks={:?} reference_peaks={:?}",
                sample.clock().elapsed(),
                branch.branch,
                branch.quantities.map(|value| value.error.rms_error),
                branch.quantities.map(|value| value.error.peak_error),
                branch.quantities.map(|value| value.error.reference_peak)
            );
        }
    }
    println!("diagnostic-only; velocity/gradient/Hessian/vorticity global samples only; no pressure, regional, arithmetic-sufficiency, convergence, or accepted-window claim; accepted_pde_windows=0");
}
