//! Exact-word and fail-closed controls for one immutable original-force slab.
use nsbu_benchmarks::{
    runtime_force::ForceSettings,
    v2_experiment::shared_force::{
        SharedForceClock, SharedForceError, SharedForceTable, SharedForceTablePlan,
    },
};
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
    integrators::forcing::PrescribedForce,
    Complex64, SolverError,
};

const CAP: usize = 256 * 1024 * 1024;

fn domain(n: usize) -> Domain {
    Domain::new([n; 3], [1.0; 3], 1.0).unwrap()
}
fn domains() -> [Domain; 3] {
    [domain(4), domain(8), domain(12)]
}
fn settings(n: usize) -> ForceSettings {
    ForceSettings {
        samples: Layout::new([n; 3]).unwrap(),
        workers: 0,
    }
}
fn clock(elapsed: u128) -> TickClock {
    TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap()
}
fn field(layout: Layout, value: Complex64) -> [Vec<Complex64>; 3] {
    std::array::from_fn(|_| vec![value; layout.half_len()])
}
fn bits(values: &[Complex64]) -> Vec<(u64, u64)> {
    values
        .iter()
        .map(|value| (value.re.to_bits(), value.im.to_bits()))
        .collect()
}
fn direct(settings: ForceSettings, domain: Domain, clock: TickClock) -> [Vec<Complex64>; 3] {
    let limits = settings.limits(domain).unwrap();
    let mut provider = settings.build(domain, limits.storage_bytes).unwrap();
    let mut output = field(domain.layout(), Complex64::new(0.0, 0.0));
    provider
        .evaluate(clock, limits, output.each_mut().map(Vec::as_mut_slice))
        .unwrap();
    output
}

fn streams() -> [Vec<TickClock>; 6] {
    let steps = [16, 16, 16, 64, 32, 16];
    steps.map(|step| {
        let mut result = Vec::new();
        for elapsed in (0..64).step_by(step as usize) {
            result.extend(clock(elapsed).stages(step).unwrap());
        }
        result
    })
}
fn slab_manifest(streams: &[Vec<TickClock>; 6]) -> Vec<SharedForceClock> {
    let branch_domains = [0, 1, 2, 2, 2, 2];
    (0..=64)
        .step_by(4)
        .map(|elapsed| {
            let mut copies = [0; 3];
            for (branch, stream) in streams.iter().enumerate() {
                copies[branch_domains[branch]] += stream
                    .iter()
                    .filter(|request| request.elapsed() == elapsed)
                    .count();
            }
            SharedForceClock::new(clock(elapsed), copies).unwrap()
        })
        .collect()
}

#[test]
fn six_branch_stage_streams_equal_fresh_same_m_force_at_every_word() {
    let streams = streams();
    let manifest = slab_manifest(&streams);
    let attempts = streams.iter().map(Vec::len).sum();
    let plan =
        SharedForceTablePlan::new(settings(16), domains(), &manifest, attempts, CAP).unwrap();
    assert_eq!(plan.bounds().work.provider_evaluations, 17);
    assert_eq!(attempts, 95);
    let mut table = SharedForceTable::new(plan).unwrap();
    let branch_domains = [0, 1, 2, 2, 2, 2];
    let mut observed_nonzero = false;
    for (branch, stream) in streams.iter().enumerate() {
        let index = branch_domains[branch];
        let retained = domains()[index];
        let binding = plan.binding(index).unwrap();
        for requested in stream {
            let expected = direct(settings(16), retained, *requested);
            let mut actual = field(retained.layout(), Complex64::new(3.0, -7.0));
            let report = table
                .copy(
                    binding,
                    *requested,
                    actual.each_mut().map(Vec::as_mut_slice),
                )
                .unwrap();
            assert_eq!(report.clock(), *requested);
            assert_eq!(report.domain(), retained);
            assert_eq!(report.identity(), plan.identity());
            assert_eq!(report.settings(), settings(16));
            for component in 0..3 {
                assert_eq!(bits(&actual[component]), bits(&expected[component]));
                assert_nyquist_zero(retained.layout(), &actual[component]);
                if requested.elapsed() > 0 {
                    observed_nonzero |= actual[component]
                        .iter()
                        .any(|value| value.re != 0.0 || value.im != 0.0);
                }
            }
        }
    }
    assert!(observed_nonzero);
    for request in &manifest {
        for retained in domains() {
            assert_eq!(table.remaining(request.clock(), retained), Some(0));
        }
    }
    assert_eq!(table.charged_work(), plan.bounds().work);
}

fn assert_nyquist_zero(layout: Layout, values: &[Complex64]) {
    for (index, value) in values.iter().enumerate() {
        let position = layout.position(index).unwrap();
        if layout.is_nyquist(position).unwrap() {
            assert_eq!(value.re.to_bits(), 0.0_f64.to_bits());
            assert_eq!(value.im.to_bits(), 0.0_f64.to_bits());
        }
    }
}

fn one_clock(copies: [usize; 3]) -> [SharedForceClock; 1] {
    [SharedForceClock::new(clock(0), copies).unwrap()]
}

#[test]
fn admission_binds_positive_ordered_manifest_resources_and_full_provider_identity() {
    assert_eq!(
        SharedForceClock::new(clock(0), [1, 0, 1]),
        Err(SolverError::InvalidPayload)
    );
    assert!(matches!(
        SharedForceTablePlan::new(settings(16), domains(), &[], 1, CAP),
        Err(SolverError::InvalidPayload)
    ));
    let reversed = [
        SharedForceClock::new(clock(4), [1; 3]).unwrap(),
        SharedForceClock::new(clock(0), [1; 3]).unwrap(),
    ];
    assert!(matches!(
        SharedForceTablePlan::new(settings(16), domains(), &reversed, 6, CAP),
        Err(SolverError::InvalidClock)
    ));
    let foreign_target = [
        SharedForceClock::new(clock(0), [1; 3]).unwrap(),
        SharedForceClock::new(TickClock::restore(-20, 16384, 4, 16380).unwrap(), [1; 3]).unwrap(),
    ];
    assert!(matches!(
        SharedForceTablePlan::new(settings(16), domains(), &foreign_target, 6, CAP),
        Err(SolverError::InvalidClock)
    ));
    let manifest = one_clock([1; 3]);
    assert!(matches!(
        SharedForceTablePlan::new(settings(16), domains(), &manifest, 2, CAP),
        Err(SolverError::ResourceLimit)
    ));
    let plan = SharedForceTablePlan::new(settings(16), domains(), &manifest, 4, CAP).unwrap();
    assert!(matches!(
        SharedForceTablePlan::new(
            settings(16),
            domains(),
            &manifest,
            4,
            plan.bounds().joint_peak_bytes - 1
        ),
        Err(SolverError::ResourceLimit)
    ));
    assert!(matches!(
        SharedForceTablePlan::new(settings(16), domains(), &manifest, usize::MAX, usize::MAX),
        Err(SolverError::SizeOverflow)
    ));
    assert!(plan.bounds().caller_manifest_bytes >= std::mem::size_of::<SharedForceClock>());
    assert!(plan.bounds().construction_peak_bytes > plan.bounds().storage_bytes);
    assert!(plan.bounds().joint_peak_bytes >= plan.bounds().construction_peak_bytes);

    let altered = SharedForceTablePlan::new(settings(24), domains(), &manifest, 4, CAP).unwrap();
    assert_ne!(plan.identity(), altered.identity());
    assert_ne!(plan.settings(), altered.settings());
}

#[test]
fn charged_refusal_preserves_output_table_and_last_complete_report() {
    let manifest = one_clock([2; 3]);
    let plan = SharedForceTablePlan::new(settings(16), domains(), &manifest, 8, CAP).unwrap();
    let mut table = SharedForceTable::new(plan).unwrap();
    let retained = domains()[0];
    let binding = plan.binding(0).unwrap();
    let mut good = field(retained.layout(), Complex64::new(5.0, 6.0));
    let first = table
        .copy(binding, clock(0), good.each_mut().map(Vec::as_mut_slice))
        .unwrap();
    let remaining = table.remaining(clock(0), retained).unwrap();
    let before = table.charged_work();
    let mut malformed = [vec![Complex64::new(8.0, 9.0); 1], vec![], vec![]];
    let original = malformed.clone();
    assert_eq!(
        table.copy(
            binding,
            clock(0),
            malformed.each_mut().map(Vec::as_mut_slice)
        ),
        Err(SharedForceError::Numerical(SolverError::InvalidPayload))
    );
    assert_eq!(malformed, original);
    assert_eq!(table.remaining(clock(0), retained), Some(remaining));
    assert_eq!(table.current().unwrap().clock(), first.clock());
    assert_eq!(table.charged_work().copy_attempts, before.copy_attempts + 1);
    let terminal = table.charged_work();
    assert_eq!(
        table.copy(
            binding,
            clock(0),
            malformed.each_mut().map(Vec::as_mut_slice)
        ),
        Err(SharedForceError::Terminated)
    );
    assert_eq!(table.charged_work(), terminal);
}

#[test]
fn foreign_unexpected_and_overconsumed_requests_fail_closed() {
    let manifest = one_clock([1; 3]);
    let plan = SharedForceTablePlan::new(settings(16), domains(), &manifest, 4, CAP).unwrap();
    let foreign = SharedForceTablePlan::new(settings(24), domains(), &manifest, 4, CAP).unwrap();
    let retained = domains()[0];

    let mut table = SharedForceTable::new(plan).unwrap();
    let mut output = field(retained.layout(), Complex64::new(2.0, 3.0));
    let original = output.clone();
    assert_eq!(
        table.copy(
            foreign.binding(0).unwrap(),
            clock(0),
            output.each_mut().map(Vec::as_mut_slice)
        ),
        Err(SharedForceError::ForeignBinding)
    );
    assert_eq!(output, original);

    let mut table = SharedForceTable::new(plan).unwrap();
    assert_eq!(
        table.copy(
            plan.binding(0).unwrap(),
            clock(4),
            output.each_mut().map(Vec::as_mut_slice)
        ),
        Err(SharedForceError::UnexpectedRequest)
    );
    assert_eq!(output, original);

    let mut table = SharedForceTable::new(plan).unwrap();
    table
        .copy(
            plan.binding(0).unwrap(),
            clock(0),
            output.each_mut().map(Vec::as_mut_slice),
        )
        .unwrap();
    let completed = output.clone();
    assert_eq!(
        table.copy(
            plan.binding(0).unwrap(),
            clock(0),
            output.each_mut().map(Vec::as_mut_slice)
        ),
        Err(SharedForceError::UnexpectedRequest)
    );
    assert_eq!(output, completed);
    assert!(table.is_terminated());
}
