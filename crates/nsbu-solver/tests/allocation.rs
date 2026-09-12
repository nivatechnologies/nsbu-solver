//! Dedicated process: allocation counters exclude test-harness and concurrent-test activity.
mod state_support;
use nsbu_solver::diagnostics::conservative::ConservativeWorkspace;
use nsbu_solver::diagnostics::sampling::SamplingWorkspace;
use nsbu_solver::domain::{Domain, Layout};
use nsbu_solver::spectral::RotationalWorkspace;
use nsbu_solver::Complex64;
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use std::alloc::System;

#[global_allocator]
static GLOBAL: &StatsAlloc<System> = &INSTRUMENTED_SYSTEM;

fn main() {
    fft();
    rotational();
    conservative();
    sampling();
    derivatives();
    physical_comparisons();
    alternate_physical_domains();
    lineage();
    prolongation();
}

fn fft() {
    use nsbu_solver::spectral::FftPlan;
    let layout = Layout::new([4, 6, 8]).unwrap();
    let reservation = FftPlan::reservation(layout).unwrap();
    let (plan, mut work) = planned(reservation, || FftPlan::new(layout, reservation).unwrap());
    let input = (0..layout.real_len())
        .map(|index| ((19 * index + 5) % 101) as f64 / 101.0)
        .collect::<Vec<_>>();
    let mut spectrum = vec![Complex64::new(0.0, 0.0); layout.half_len()];
    let mut output = vec![0.0; layout.real_len()];
    plan.forward(&input, &mut spectrum, &mut work).unwrap();
    plan.inverse(&spectrum, &mut output, &mut work).unwrap();
    let region = Region::new(GLOBAL);
    for _ in 0..20 {
        plan.forward(&input, &mut spectrum, &mut work).unwrap();
        plan.inverse(&spectrum, &mut output, &mut work).unwrap();
    }
    assert!(input
        .iter()
        .zip(&output)
        .all(|(a, b)| (a - b).abs() < 3e-14));
    no_allocations(region.change());
}

fn alternate_physical_domains() {
    use nsbu_solver::diagnostics::physical::{
        PhysicalComparisonWorkspace, PhysicalField, PhysicalQuantity,
    };
    let small = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let fine = Domain::new([8; 3], [1.0; 3], 1.0).unwrap();
    let samples = Layout::new([12; 3]).unwrap();
    let bytes = PhysicalComparisonWorkspace::reservation(fine, fine, samples).unwrap();
    let mut workspace = planned(bytes, || {
        PhysicalComparisonWorkspace::new(fine, fine, samples, bytes).unwrap()
    });
    let coarse = vec![Complex64::new(0.0, 0.0); small.layout().half_len()];
    let value = vec![Complex64::new(0.0, 0.0); fine.layout().half_len()];
    let left = PhysicalField::Vector([&coarse; 3]);
    let right = PhysicalField::Vector([&value; 3]);
    let region = Region::new(GLOBAL);
    assert!(workspace
        .compare_domains([fine, small], right, left, PhysicalQuantity::Gradient, 1.0)
        .is_err());
    assert_eq!(
        workspace
            .compare_domains([small, fine], left, right, PhysicalQuantity::Gradient, 1.0)
            .unwrap()
            .domains(),
        [small, fine]
    );
    assert_eq!(
        workspace
            .compare(right, right, PhysicalQuantity::Vector, 1.0)
            .unwrap()
            .domains(),
        [fine; 2]
    );
    no_allocations(region.change());
}

fn physical_comparisons() {
    use nsbu_solver::diagnostics::physical::{
        PhysicalComparisonWorkspace, PhysicalField, PhysicalQuantity,
    };
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let layout = Layout::new([6; 3]).unwrap();
    let bytes = PhysicalComparisonWorkspace::reservation(domain, domain, layout).unwrap();
    let refused = Region::new(GLOBAL);
    assert!(PhysicalComparisonWorkspace::new(domain, domain, layout, bytes - 1).is_err());
    no_allocations(refused.change());
    let mut work = planned(bytes, || {
        PhysicalComparisonWorkspace::new(domain, domain, layout, bytes).unwrap()
    });
    let zero = vec![Complex64::new(0.0, 0.0); domain.layout().half_len()];
    let region = Region::new(GLOBAL);
    for quantity in [
        PhysicalQuantity::Scalar,
        PhysicalQuantity::ScalarGradient,
        PhysicalQuantity::Vector,
        PhysicalQuantity::Gradient,
        PhysicalQuantity::Hessian,
        PhysicalQuantity::Vorticity,
    ] {
        let field = if matches!(
            quantity,
            PhysicalQuantity::Scalar | PhysicalQuantity::ScalarGradient
        ) {
            PhysicalField::Scalar(&zero)
        } else {
            PhysicalField::Vector([&zero; 3])
        };
        assert!(work.compare(field, field, quantity, 0.0).is_err());
        let result = work.compare(field, field, quantity, 1.0).unwrap();
        assert_eq!(result.global().rms_error, 0.0);
    }
    no_allocations(region.change());
}

fn prolongation() {
    use nsbu_solver::domain::{Epoch, ExtraStorage, ResourcePlan, SpectralState, TickClock};
    use nsbu_solver::lineage::ProlongedState;
    let source = SpectralState::from_rest(
        state_support::plan(Epoch(0)),
        TickClock::from_rest(-12, 100).unwrap(),
        Epoch(0),
    )
    .unwrap();
    let target = ResourcePlan::new(
        Domain::new([8; 3], [1.0; 3], 1.0).unwrap(),
        ExtraStorage {
            fft: 0,
            force: 0,
            diagnostics: 0,
            overhead: 4096,
        },
        1 << 22,
        Epoch(1),
    )
    .unwrap();
    let bytes = ProlongedState::reservation(target).unwrap();
    let refused = Region::new(GLOBAL);
    assert!(ProlongedState::from_state(&source, target, bytes - 1).is_err());
    no_allocations(refused.change());
    let image = planned(bytes, || {
        ProlongedState::from_state(&source, target, bytes).unwrap()
    });
    let moving = Region::new(GLOBAL);
    let restored = image.into_state();
    assert_eq!(restored.clock(), source.clock());
    no_allocations(moving.change());
}

fn rotational() {
    let domain = Domain::new([8; 3], [1.0; 3], 1.0).unwrap();
    let reservation = RotationalWorkspace::reservation(domain).unwrap();
    let mut work = planned(reservation, || {
        RotationalWorkspace::new(domain, reservation).unwrap()
    });
    let zero = vec![Complex64::new(0.0, 0.0); domain.layout().half_len()];
    let mut output = [zero.clone(), zero.clone(), zero.clone()];
    let mut pressure = zero.clone();
    let region = Region::new(GLOBAL);
    for _ in 0..20 {
        let [a, b, c] = &mut output;
        work.evaluate([&zero; 3], [&zero; 3], [a, b, c], &mut pressure)
            .unwrap();
    }
    no_allocations(region.change());
}

fn conservative() {
    let domain = Domain::new([8; 3], [1.0; 3], 1.0).unwrap();
    let diagnostic = ConservativeWorkspace::diagnostic_domain(domain).unwrap();
    let reservation = ConservativeWorkspace::reservation(domain).unwrap();
    let mut work = planned(reservation, || {
        ConservativeWorkspace::new(domain, reservation).unwrap()
    });
    let velocity = vec![Complex64::new(0.0, 0.0); domain.layout().half_len()];
    let force = vec![Complex64::new(0.0, 0.0); diagnostic.layout().half_len()];
    let mut output = [force.clone(), force.clone(), force.clone()];
    let mut pressure = force.clone();
    let region = Region::new(GLOBAL);
    for _ in 0..20 {
        let [a, b, c] = &mut output;
        work.evaluate([&velocity; 3], [&force; 3], [a, b, c], &mut pressure)
            .unwrap();
    }
    no_allocations(region.change());
}

fn no_allocations(stats: stats_alloc::Stats) {
    assert_eq!(stats.allocations, 0);
    assert_eq!(stats.deallocations, 0);
    assert_eq!(stats.reallocations, 0);
    assert_eq!(stats.bytes_allocated, 0);
    assert_eq!(stats.bytes_deallocated, 0);
}

fn sampling() {
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let layout = Layout::new([6; 3]).unwrap();
    let reservation = SamplingWorkspace::reservation(domain, layout).unwrap();
    let mut work = planned(reservation, || {
        SamplingWorkspace::new(domain, layout, reservation).unwrap()
    });
    let zero = vec![Complex64::new(0.0, 0.0); domain.layout().half_len()];
    let region = Region::new(GLOBAL);
    for _ in 0..20 {
        let samples = work.sample([&zero; 3]).unwrap();
        assert_eq!(samples.velocity_maximum.value, 0.0);
        assert_eq!(samples.vorticity_maximum.value, 0.0);
    }
    no_allocations(region.change());
}

fn derivatives() {
    use nsbu_solver::diagnostics::derivatives::{Derivative, DerivativeWorkspace};
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let layout = Layout::new([6; 3]).unwrap();
    let reservation = DerivativeWorkspace::reservation(domain, layout).unwrap();
    let refusal = Region::new(GLOBAL);
    assert!(DerivativeWorkspace::new(domain, layout, reservation - 1).is_err());
    no_allocations(refusal.change());
    let mut work = planned(reservation, || {
        DerivativeWorkspace::new(domain, layout, reservation).unwrap()
    });
    let zero = vec![Complex64::new(0.0, 0.0); domain.layout().half_len()];
    let region = Region::new(GLOBAL);
    for _ in 0..20 {
        for orders in [[0, 0, 0], [1, 0, 0], [1, 1, 0], [0, 0, 2]] {
            let derivative = Derivative::new(orders).unwrap();
            assert!(work.sample(&zero[..1], derivative).is_err());
            let samples = work.sample(&zero, derivative).unwrap();
            assert!(samples.values.iter().all(|&value| value == 0.0));
        }
    }
    no_allocations(region.change());
}

fn planned<T>(reservation: usize, allocate: impl FnOnce() -> T) -> T {
    let region = Region::new(GLOBAL);
    let value = allocate();
    let stats = region.change();
    assert!(stats.bytes_allocated <= reservation);
    assert_eq!(stats.reallocations, 0);
    value
}

fn lineage() {
    use nsbu_solver::domain::{Epoch, SpectralState, TickClock};
    use nsbu_solver::lineage::{Digest, Origin, PhysicalImage, Profile, Registry};
    let plan = state_support::plan(Epoch(0));
    let clock = TickClock::from_rest(-12, 100).unwrap();
    let state = SpectralState::from_rest(plan, clock, Epoch(0)).unwrap();
    let bytes = PhysicalImage::reservation(plan).unwrap();
    let refusal = Region::new(GLOBAL);
    assert!(PhysicalImage::capture(&state, bytes - 1).is_err());
    no_allocations(refusal.change());
    let image = planned(bytes, || PhysicalImage::capture(&state, bytes).unwrap());
    let region = Region::new(GLOBAL);
    let restored = image.into_state();
    assert_eq!(restored.clock(), clock);
    let mut slots = [None; 8];
    let identity = Digest::new([1; 32]).unwrap();
    let mut registry = Registry::new(identity, &mut slots, 8).unwrap();
    let profile = Profile {
        problem: identity,
        execution: identity,
        force: identity,
    };
    let id = registry.append(profile, clock, Origin::FromRest).unwrap();
    assert!(registry.get(id).unwrap().is_direct());
    assert_eq!(registry.invalidate_force(identity, 1), Ok(1));
    no_allocations(region.change());
}
