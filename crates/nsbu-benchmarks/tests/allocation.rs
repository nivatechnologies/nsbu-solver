//! Provider heap accounting and nonmonotone stage requests in an isolated process.
use nsbu_benchmarks::provider::V2Force;
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
    integrators::forcing::PrescribedForce,
    Complex64,
};
use stats_alloc::Region;

#[global_allocator]
static GLOBAL: &stats_alloc::StatsAlloc<std::alloc::System> = &stats_alloc::INSTRUMENTED_SYSTEM;

mod owned_reconstruction_support;
mod probe_allocation_support;
mod reconstruction_observer_support;
mod smooth_family_support;

fn main() {
    probe_allocation_support::check();
    regional_derivatives();
    physical_regions();
    physical_refinement_family();
    pressure_refinement_family();
    reconstruction_accepted_ring();
    owned_reconstruction_restart();
    reconstruction_archive_restart();
    independent_family_comparisons();
    independent_residual_probes();
    verified_replay_allocation();
    smooth_admission_and_attempts();
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let sampled = Layout::new([6; 3]).unwrap();
    let limits = V2Force::preflight(domain, sampled).unwrap();
    let planning = Region::new(GLOBAL);
    let mut provider = V2Force::new(domain, sampled, limits.storage_bytes).unwrap();
    let allocated = planning.change();
    assert_eq!(allocated.allocations, 11);
    assert_eq!(allocated.reallocations, 0);
    let object_bytes = std::mem::size_of::<V2Force>()
        + std::mem::size_of::<nsbu_solver::spectral::FftPlan>()
        + std::mem::size_of::<nsbu_solver::spectral::FftWorkspace>();
    assert_eq!(
        limits.storage_bytes,
        allocated.bytes_allocated + object_bytes + 11 * 64
    );
    let mut buffers: [Vec<Complex64>; 3] =
        std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); domain.layout().half_len()]);
    let mut first: [Vec<Complex64>; 3] = buffers.clone();
    let region = Region::new(GLOBAL);
    for elapsed in [1, 4, 2, 3, 0, 1] {
        let clock = TickClock::restore(-10, 8, elapsed, 8 - elapsed).unwrap();
        let [a, b, c] = &mut buffers;
        let work = provider.evaluate(clock, limits, [a, b, c]).unwrap();
        assert!(work.work_units <= limits.work_units);
        for (saved, current) in first.iter_mut().zip(&buffers) {
            if elapsed == 1 {
                if saved.iter().all(|v| *v == Complex64::new(0.0, 0.0)) {
                    saved.copy_from_slice(current);
                }
                assert_eq!(saved, current);
            }
        }
    }
    let actual = region.change();
    assert_eq!(
        (
            actual.allocations,
            actual.deallocations,
            actual.reallocations
        ),
        (0, 0, 0)
    );
}

fn physical_refinement_family() {
    use nsbu_benchmarks::smooth_experiment::{
        physical::{PhysicalFamilyPlan, PhysicalFamilyWorkspace},
        FamilyPlan, SmoothFamily,
    };
    use nsbu_solver::verification::times::TestedTimes;
    let clocks = smooth_family_support::clocks();
    let times = TestedTimes::new(&clocks, 3).unwrap();
    let family_plan = FamilyPlan::new(
        smooth_family_support::settings(1e-2),
        times,
        smooth_family_support::CAP,
    )
    .unwrap();
    let samples = Layout::new([12; 3]).unwrap();
    let refusal = Region::new(GLOBAL);
    assert!(PhysicalFamilyPlan::new(family_plan, samples, [1.0; 4], 4, 1).is_err());
    assert_eq!(refusal.change().allocations, 0);
    let plan = PhysicalFamilyPlan::new(
        family_plan,
        samples,
        [1.0; 4],
        4,
        smooth_family_support::CAP,
    )
    .unwrap();
    let allocation = Region::new(GLOBAL);
    let mut family = SmoothFamily::new(family_plan).unwrap();
    let mut physical = PhysicalFamilyWorkspace::new(plan).unwrap();
    assert!(allocation.change().bytes_allocated <= plan.bounds().joint_storage_bytes);
    let region = Region::new(GLOBAL);
    assert!(physical.measure(&family).is_err());
    family.advance().unwrap();
    let result = physical.measure(&family).unwrap();
    assert_eq!(result.clock(), clocks[0]);
    assert_eq!(physical.charged_work().scalar_transforms, 900);
    let measured = region.change();
    assert_eq!(measured.allocations, 0);
    assert_eq!(measured.reallocations, 0);
    assert_eq!(measured.deallocations, 0);
}

fn physical_regions() {
    use nsbu_benchmarks::regions::physical;
    use nsbu_solver::diagnostics::physical::{
        PhysicalComparisonWorkspace, PhysicalField, PhysicalQuantity,
    };
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let samples = Layout::new([6; 3]).unwrap();
    let bytes = PhysicalComparisonWorkspace::reservation(domain, domain, samples).unwrap();
    let mut workspace = PhysicalComparisonWorkspace::new(domain, domain, samples, bytes).unwrap();
    let zero = vec![Complex64::new(0.0, 0.0); domain.layout().half_len()];
    let clock = TickClock::restore(-20, 8192, 4096, 4096).unwrap();
    let region = Region::new(GLOBAL);
    let fields = PhysicalField::Vector([&zero; 3]);
    let input = workspace
        .compare(fields, fields, PhysicalQuantity::Hessian, 1.0)
        .unwrap();
    assert!(physical::measure(&input, clock, 128, samples.real_len() - 1).is_err());
    let report = physical::measure(&input, clock, 128, samples.real_len()).unwrap();
    assert!(report.grid_complete);
    assert_eq!(report.components, 27);
    let measured = region.change();
    assert_eq!(measured.allocations, 0);
    assert_eq!(measured.reallocations, 0);
    assert_eq!(measured.deallocations, 0);
}

fn regional_derivatives() {
    use nsbu_benchmarks::{fields::reference, regions::RegionalTensorErrors, time::BenchmarkTime};
    let clock = TickClock::restore(-20, 8192, 4096, 4096).unwrap();
    let time = BenchmarkTime::new(clock).unwrap();
    let layout = Layout::new([4; 3]).unwrap();
    let region = Region::new(GLOBAL);
    let mut errors = RegionalTensorErrors::<27>::new(clock, layout, 128, 64, 1.0).unwrap();
    while let Some(point) = errors.next_point() {
        let values = reference::evaluate(point, time).unwrap();
        let tensor = std::array::from_fn(|i| values.hessian[i / 9][(i / 3) % 3][i % 3]);
        errors.push([0.0; 27], tensor).unwrap();
    }
    assert!(errors.report().unwrap().grid_complete);
    let measured = region.change();
    assert_eq!(measured.allocations, 0);
    assert_eq!(measured.reallocations, 0);
    assert_eq!(measured.deallocations, 0);
}

fn smooth_admission_and_attempts() {
    use nsbu_benchmarks::smooth_run::{archive, SmoothPlan, SmoothRun};
    use nsbu_solver::{
        experiment::control::Configuration,
        integrators::{indicator::Tolerances, method::Method, trajectory::RunLimits},
    };
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let clock = TickClock::from_rest(-20, 1 << 20).unwrap();
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        let configuration = Configuration {
            limits: RunLimits {
                endpoint: 2048,
                step_ticks: 1024,
                maximum_attempts: 2,
            },
            method,
            tolerances: Tolerances {
                absolute: [1e-2; 2],
                relative: [0.0; 2],
            },
        };
        let admission = Region::new(GLOBAL);
        let plan = SmoothPlan::from_rest(domain, clock, configuration, 2, 0.3, 1 << 23).unwrap();
        assert!(SmoothPlan::from_rest(domain, clock, configuration, 2, 0.3, 1).is_err());
        let allocation = admission.change();
        assert_eq!((allocation.allocations, allocation.reallocations), (0, 0));
        let construction = Region::new(GLOBAL);
        let mut run = SmoothRun::from_rest(
            domain,
            clock,
            configuration,
            2,
            0.3,
            plan.resources().total(),
        )
        .unwrap();
        assert!(construction.change().bytes_allocated <= plan.resources().total());
        let stepping = Region::new(GLOBAL);
        run.step().unwrap();
        let first_allocation = stepping.change();
        assert_eq!(
            (first_allocation.allocations, first_allocation.reallocations),
            (0, 0)
        );
        let mut bytes = vec![0; archive::encoded_len(&run).unwrap()];
        archive::write(&run, &mut bytes).unwrap();
        let imported = archive::read(&bytes, run.state().plan(), bytes.len(), 1 << 23).unwrap();
        let mut resumed = imported.continue_unverified(1 << 23).unwrap();
        let stepping = Region::new(GLOBAL);
        let resumed_outcome = resumed.step();
        assert_eq!(resumed_outcome, run.step());
        let allocation = stepping.change();
        assert_eq!(
            (
                allocation.allocations,
                allocation.reallocations,
                allocation.deallocations
            ),
            (0, 0, 0)
        );
        assert_eq!(run.history().controller().committed(), 2);
    }
}

fn reconstruction_accepted_ring() {
    use nsbu_solver::integrators::method::Method;
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        let mut case = reconstruction_observer_support::Case::new(method, 5, 1e-2);
        let mut value =
            vec![Complex64::new(0.0, 0.0); case.state.plan().domain().layout().half_len()];
        let mut derivative = value.clone();
        let region = Region::new(GLOBAL);
        for _ in 0..3 {
            case.step();
        }
        let probe = TickClock::restore(-20, 1 << 20, 2049, (1 << 20) - 2049).unwrap();
        case.observer
            .reconstruct(probe, 0, &mut value, &mut derivative)
            .unwrap();
        let measured = region.change();
        assert_eq!(
            (
                measured.allocations,
                measured.reallocations,
                measured.deallocations
            ),
            (0, 0, 0)
        );
        assert_eq!(
            case.observer
                .last_accepted_clocks()
                .unwrap()
                .map(|t| t.elapsed()),
            [1024, 2048, 3072]
        );
    }
}

fn owned_reconstruction_restart() {
    use nsbu_benchmarks::smooth_run::ReconstructedRun;
    use nsbu_solver::integrators::method::Method;
    use owned_reconstruction_support::{configuration, run, CAP};
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        let construction = Region::new(GLOBAL);
        let mut original = run(method, 1e-2, 5);
        assert!(construction.change().bytes_allocated <= original.state().plan().total());
        original.step().unwrap();
        original.step().unwrap();
        let cap = original.snapshot_reservation().unwrap();
        let refused = Region::new(GLOBAL);
        assert!(original.snapshot(cap - 1).is_err());
        assert_eq!(refused.change().allocations, 0);
        let capture = Region::new(GLOBAL);
        let snapshot = original.snapshot(cap).unwrap();
        assert!(capture.change().bytes_allocated <= cap);
        let mut restored =
            ReconstructedRun::restore(snapshot, configuration(method, 1e-2), CAP).unwrap();
        let mut value =
            vec![Complex64::new(0.0, 0.0); original.state().plan().domain().layout().half_len()];
        let mut derivative = value.clone();
        let stepping = Region::new(GLOBAL);
        assert_eq!(original.step(), restored.step());
        let probe = TickClock::restore(-20, 1 << 20, 2049, (1 << 20) - 2049).unwrap();
        restored
            .observer()
            .reconstruct(probe, 0, &mut value, &mut derivative)
            .unwrap();
        let measured = stepping.change();
        assert_eq!(
            (
                measured.allocations,
                measured.reallocations,
                measured.deallocations
            ),
            (0, 0, 0)
        );
    }
}

fn reconstruction_archive_restart() {
    use nsbu_benchmarks::smooth_run::reconstructed_archive as archive;
    use nsbu_solver::integrators::method::Method;
    use owned_reconstruction_support::{run, CAP};
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        let mut original = run(method, 1e-2, 5);
        original.step().unwrap();
        original.step().unwrap();
        let mut bytes = vec![0; archive::encoded_len(&original).unwrap()];
        let encoding = Region::new(GLOBAL);
        archive::write(&original, &mut bytes).unwrap();
        assert_eq!(encoding.change().allocations, 0);
        let refused = Region::new(GLOBAL);
        assert!(archive::read(&bytes, original.state().plan(), bytes.len(), 1).is_err());
        assert_eq!(refused.change().allocations, 0);
        let decoding = Region::new(GLOBAL);
        let imported = archive::read(&bytes, original.state().plan(), bytes.len(), CAP).unwrap();
        let mut restored = imported.continue_unverified(CAP).unwrap();
        assert!(decoding.change().bytes_allocated <= CAP);
        let mut value =
            vec![Complex64::new(0.0, 0.0); original.state().plan().domain().layout().half_len()];
        let mut derivative = value.clone();
        let stepping = Region::new(GLOBAL);
        assert_eq!(original.step(), restored.step());
        let probe = TickClock::restore(-20, 1 << 20, 2049, (1 << 20) - 2049).unwrap();
        restored
            .observer()
            .reconstruct(probe, 0, &mut value, &mut derivative)
            .unwrap();
        let measured = stepping.change();
        assert_eq!(
            (
                measured.allocations,
                measured.reallocations,
                measured.deallocations
            ),
            (0, 0, 0)
        );
    }
}

fn independent_family_comparisons() {
    use nsbu_benchmarks::smooth_experiment::{
        reconstruction::ReconstructionWorkspace, FamilyPlan, SmoothFamily,
    };
    use nsbu_solver::verification::times::TestedTimes;
    use smooth_family_support::{clocks, settings, CAP};
    let clocks = clocks();
    let times = TestedTimes::new(&clocks, 3).unwrap();
    let admission = Region::new(GLOBAL);
    let plan = FamilyPlan::new(settings(1e-2), times, CAP).unwrap();
    assert!(FamilyPlan::new(settings(1e-2), times, 1).is_err());
    assert_eq!(admission.change().allocations, 0);
    let construction = Region::new(GLOBAL);
    let mut family = SmoothFamily::new(plan).unwrap();
    assert!(construction.change().bytes_allocated <= plan.bounds().storage_bytes);
    let mut reconstruction = ReconstructionWorkspace::new(plan, 1, CAP).unwrap();
    let comparisons = Region::new(GLOBAL);
    while family.advance().unwrap().is_some() {}
    let probe = TickClock::restore(-16, 512, 127, 385).unwrap();
    reconstruction.measure(&family, probe).unwrap();
    let measured = comparisons.change();
    assert_eq!(
        (
            measured.allocations,
            measured.reallocations,
            measured.deallocations
        ),
        (0, 0, 0)
    );
}

fn independent_residual_probes() {
    use nsbu_benchmarks::smooth_experiment::residual::ResidualWorkspace;
    use nsbu_solver::integrators::method::Method;
    let mut run = owned_reconstruction_support::run(Method::CoxMatthews, 1e-2, 5);
    run.step().unwrap();
    run.step().unwrap();
    let domain = run.state().plan().domain();
    let admitted = Region::new(GLOBAL);
    let bounds = ResidualWorkspace::reservation(domain, 2).unwrap();
    assert!(ResidualWorkspace::new(domain, 2, 1).is_err());
    let stat = admitted.change();
    assert_eq!(
        (stat.allocations, stat.reallocations, stat.deallocations),
        (0, 0, 0)
    );
    let construction = Region::new(GLOBAL);
    let mut diagnostic = ResidualWorkspace::new(domain, 2, bounds.storage_bytes).unwrap();
    assert!(construction.change().bytes_allocated <= bounds.storage_bytes);
    let probe = TickClock::restore(-20, 1 << 20, 769, (1 << 20) - 769).unwrap();
    let measured = Region::new(GLOBAL);
    for _ in 0..2 {
        assert!(diagnostic.measure(&run, probe).unwrap().norms().h1 < 1e-4);
    }
    assert!(diagnostic.measure(&run, probe).is_err());
    let stat = measured.change();
    assert_eq!(
        (stat.allocations, stat.reallocations, stat.deallocations),
        (0, 0, 0)
    );
}

fn verified_replay_allocation() {
    use nsbu_benchmarks::smooth_run::replay::ReplayPlan;
    use nsbu_solver::integrators::method::Method;
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        let mut run = owned_reconstruction_support::run(method, 1e-2, 5);
        run.step().unwrap();
        run.step().unwrap();
        let admission = Region::new(GLOBAL);
        let plan = ReplayPlan::new(&run, 2, owned_reconstruction_support::CAP).unwrap();
        assert!(ReplayPlan::new(&run, 1, owned_reconstruction_support::CAP).is_err());
        assert!(ReplayPlan::new(&run, 2, 1).is_err());
        let stat = admission.change();
        assert_eq!(
            (stat.allocations, stat.reallocations, stat.deallocations),
            (0, 0, 0)
        );
        let bound = plan.bounds();
        let measured = Region::new(GLOBAL);
        let replayed = plan.execute().unwrap();
        let stat = measured.change();
        assert_eq!(stat.reallocations, 0);
        assert!(stat.bytes_allocated <= bound.storage_bytes);
        assert_eq!(replayed.report().attempts, 2);
        owned_reconstruction_support::compare(replayed.run(), &run);
    }
}

fn pressure_refinement_family() {
    use nsbu_benchmarks::smooth_experiment::{
        pressure::{PressureFamilyPlan, PressureFamilyWorkspace},
        FamilyPlan, SmoothFamily,
    };
    use nsbu_solver::verification::times::TestedTimes;
    let clocks = smooth_family_support::clocks();
    let times = TestedTimes::new(&clocks, 3).unwrap();
    let family_plan = FamilyPlan::new(
        smooth_family_support::settings(1e-2),
        times,
        smooth_family_support::CAP,
    )
    .unwrap();
    let samples = Layout::new([24; 3]).unwrap();
    let refusal = Region::new(GLOBAL);
    assert!(PressureFamilyPlan::new(family_plan, samples, [1.0; 2], 4, 1).is_err());
    assert_eq!(refusal.change().allocations, 0);
    let plan = PressureFamilyPlan::new(
        family_plan,
        samples,
        [1.0; 2],
        4,
        smooth_family_support::CAP,
    )
    .unwrap();
    let allocation = Region::new(GLOBAL);
    let mut family = SmoothFamily::new(family_plan).unwrap();
    let mut physical = PressureFamilyWorkspace::new(plan).unwrap();
    assert!(allocation.change().bytes_allocated <= plan.bounds().joint_storage_bytes);
    let region = Region::new(GLOBAL);
    assert!(physical.measure(&family).is_err());
    family.advance().unwrap();
    let result = physical.measure(&family).unwrap();
    assert_eq!(result.clock(), clocks[0]);
    assert_eq!(physical.charged_work().scalar_transforms, 260);
    let measured = region.change();
    assert_eq!(measured.allocations, 0);
    assert_eq!(measured.reallocations, 0);
    assert_eq!(measured.deallocations, 0);
}
