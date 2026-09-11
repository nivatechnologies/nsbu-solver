//! Bounded accepted-node binding across ordinary and lookahead probe families.
use nsbu_benchmarks::{
    runtime_force::ForceSettings,
    v2_experiment::{
        binding::{NodeBindingPlan, NodeBindingStatus, NodeBindingWorkspace},
        probes::{ProbeFamily, ProbePlan},
        FamilyPlan, FamilySettings, V2Family,
    },
};
use nsbu_solver::{
    domain::{Layout, TickClock},
    integrators::indicator::Tolerances,
    verification::times::TestedTimes,
};

fn main() {
    let accepted =
        [0, 64, 128].map(|elapsed| TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap());
    let probe_times =
        [0, 63, 128].map(|elapsed| TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap());
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
        TestedTimes::new(&accepted, accepted.len()).unwrap(),
        cap,
    )
    .unwrap();
    let probe_plan = ProbePlan::new(
        family_plan,
        TestedTimes::new(&probe_times, probe_times.len()).unwrap(),
        probe_times.len(),
        cap,
    )
    .unwrap();
    let binding_plan = NodeBindingPlan::new(family_plan, probe_plan, accepted.len(), cap).unwrap();
    println!("node_binding_preflight={:?}", binding_plan.bounds());
    let mut family = V2Family::new(family_plan).unwrap();
    let mut probes = ProbeFamily::new(probe_plan).unwrap();
    let mut binding = NodeBindingWorkspace::new(binding_plan);

    family.advance().unwrap();
    probes.advance().unwrap();
    probes.advance().unwrap();
    print(binding.measure(&family, &probes).unwrap());
    family.advance().unwrap();
    probes.advance().unwrap();
    print(binding.measure(&family, &probes).unwrap());
    family.advance().unwrap();
    print(binding.measure(&family, &probes).unwrap());
    println!("diagnostic provenance/equality only; missing is not zero, false is not a bound; no reconstructed-state substitution, tolerance, convergence or PDE-window claim; accepted_pde_windows=0");
}

fn print(sample: nsbu_benchmarks::v2_experiment::binding::NodeBindingSample) {
    let labels = sample.branches().map(|status| match status {
        NodeBindingStatus::MissingRetainedNode => "missing",
        NodeBindingStatus::Compared(node) if node.coefficients_equal => "bitwise-equal",
        NodeBindingStatus::Compared(_) => "different",
    });
    println!(
        "elapsed={} branch_bindings={labels:?}",
        sample.clock().elapsed()
    );
}
