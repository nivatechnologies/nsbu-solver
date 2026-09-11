//! Bounded global and sampled-region tracking of six actual exact-v2 trajectories.
use nsbu_benchmarks::{
    runtime_force::ForceSettings,
    v2_experiment::{
        reference::{
            regional::{RegionalTrackingPlan, RegionalTrackingWorkspace},
            ReferenceTrackingPlan,
        },
        FamilyPlan, FamilySettings, V2Family,
    },
    CASE_SHA256,
};
use nsbu_solver::{
    diagnostics::local::SampledError,
    domain::{Layout, TickClock},
    integrators::indicator::Tolerances,
    verification::times::TestedTimes,
};

fn main() {
    let clocks =
        [0, 64, 128].map(|elapsed| TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap());
    let settings = FamilySettings {
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
        settings,
        TestedTimes::new(&clocks, clocks.len()).unwrap(),
        cap,
    )
    .unwrap();
    let tracking = ReferenceTrackingPlan::new(
        family_plan,
        Layout::new([12; 3]).unwrap(),
        [1e-8, 1e-7, 1e-6, 1e-7],
        clocks.len(),
        cap,
    )
    .unwrap();
    let plan = RegionalTrackingPlan::new(tracking, 128, cap).unwrap();
    println!("case=similarity-mms-v2 sha256={CASE_SHA256}");
    println!(
        "family_identity={:02x?} regional_tracking_preflight={:?}",
        family_plan.identity(),
        plan.bounds()
    );
    let mut family = V2Family::new(family_plan).unwrap();
    let mut observer = RegionalTrackingWorkspace::new(plan).unwrap();
    while let Some(family_sample) = family.advance().unwrap() {
        let sample = observer.measure(&family).unwrap();
        assert_eq!(sample.clock(), family_sample.clock());
        for branch in sample.branches() {
            for finding in branch.quantities {
                let regional: Vec<_> = finding
                    .regional
                    .regions
                    .map(|(region, result)| match result {
                        SampledError::Measured(error) => {
                            (region, Some((error.samples, error.rms_error)))
                        }
                        SampledError::NoSamples => (region, None),
                    })
                    .into();
                println!(
                    "elapsed={} branch={} quantity={:?} global_RMS={} regions={regional:?}",
                    sample.clock().elapsed(),
                    branch.branch,
                    finding.quantity,
                    finding.global.rms_error,
                );
            }
        }
    }
    println!("diagnostic-only sampled global and geometric classes; no pressure/gauge, arbitrary nominal-set coverage, current-grid arithmetic bound, continuum bound, convergence or PDE-window claim; accepted_pde_windows=0");
}
