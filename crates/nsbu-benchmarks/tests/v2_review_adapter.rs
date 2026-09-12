//! One actual schedule checks mapping, fail-closed controls and steady allocation.
mod v2_family_support;

use nsbu_benchmarks::v2_experiment::{
    diagnostic::{DiagnosticDriver, DiagnosticEvent, DiagnosticPlan, DiagnosticSettings},
    probes::ProbePlan,
    review_adapter::{
        extract, AdapterBounds, AdapterError, AdapterRecord, AdapterStatus, RecordAvailability,
        ReviewProfile, OBSERVABLES, REVIEW_RECORDS,
    },
    FamilyPlan,
};
use nsbu_benchmarks::CASE_SHA256;
use nsbu_solver::{
    diagnostics::physical::PhysicalQuantity,
    domain::{Layout, TickClock},
    verification::{
        budget::{Channel, CHANNELS},
        refinement::Evidence,
        times::TestedTimes,
    },
};
use stats_alloc::{Region, StatsAlloc, INSTRUMENTED_SYSTEM};
use v2_family_support::{clocks, settings, CAP};

#[global_allocator]
static GLOBAL: &StatsAlloc<std::alloc::System> = &INSTRUMENTED_SYSTEM;

fn ticks(values: &[u128]) -> Vec<TickClock> {
    values
        .iter()
        .map(|&elapsed| TickClock::restore(-20, 8192, elapsed, 8192 - elapsed).unwrap())
        .collect()
}

fn settings_policy() -> DiagnosticSettings {
    DiagnosticSettings {
        physical_samples: Layout::new([12; 3]).unwrap(),
        pressure_samples: Layout::new([24; 3]).unwrap(),
        reference_samples: Layout::new([12; 3]).unwrap(),
        physical_floors: [1e-8, 1e-7, 1e-6, 1e-7],
        pressure_floors: [1e-8, 1e-7],
        reference_floors: [1e-8, 1e-7, 1e-6, 1e-7],
        regional_root_budget: 128,
        coverage_panels: [256, 512, 1024],
    }
}

fn plan<'a>(
    accepted: &'a [TickClock],
    manifest: &'a [TickClock],
    residual: &'a [TickClock],
    tolerance: f64,
) -> DiagnosticPlan<'a> {
    let family = FamilyPlan::new(
        settings(tolerance),
        TestedTimes::new(accepted, accepted.len()).unwrap(),
        CAP,
    )
    .unwrap();
    let probes = ProbePlan::new(
        family,
        TestedTimes::new(manifest, manifest.len()).unwrap(),
        manifest.len(),
        CAP,
    )
    .unwrap();
    DiagnosticPlan::new(family, probes, residual, settings_policy(), CAP).unwrap()
}

fn profile() -> ReviewProfile {
    ReviewProfile {
        physical_samples: Layout::new([12; 3]).unwrap(),
        tracking_samples: Layout::new([12; 3]).unwrap(),
        relative_floors: [1e-8, 1e-7, 1e-6, 1e-7],
    }
}

fn assert_missing(evidence: Evidence) {
    assert!(matches!(evidence, Evidence::Missing));
}

fn assert_bits(actual: f64, expected: f64) {
    assert_eq!(actual.to_bits(), expected.to_bits());
}

fn assert_accepted(record: AdapterRecord, event: DiagnosticEvent, quantity: usize) {
    assert_eq!(record.availability, RecordAvailability::AcceptedMeasured);
    assert!(record.reconstruction.is_none());
    let accepted = event.accepted().sample().unwrap();
    let physical = accepted.physical.quantities()[quantity];
    let tracking = accepted.regional_reference.branches()[2].quantities[quantity];
    assert_bits(record.tracking_error.unwrap(), tracking.global.rms_error);
    let Evidence::Sequence(space) = record.channels[Channel::Space as usize] else {
        panic!("space was not a measured sequence")
    };
    let Evidence::Sequence(time) = record.channels[Channel::Time as usize] else {
        panic!("time was not a measured sequence")
    };
    let Evidence::Pair(method) = record.channels[Channel::Method as usize] else {
        panic!("method was not measured")
    };
    for (actual, expected) in space
        .into_iter()
        .zip(physical.space.map(|value| value.rms_error))
    {
        assert_bits(actual, expected);
    }
    for (actual, expected) in time
        .into_iter()
        .zip(physical.time.map(|value| value.rms_error))
    {
        assert_bits(actual, expected);
    }
    assert_bits(method, physical.method.rms_error);
    for channel in CHANNELS
        .into_iter()
        .filter(|channel| !matches!(channel, Channel::Space | Channel::Time | Channel::Method))
    {
        assert_missing(record.channels[channel as usize]);
    }
}

fn assert_offstage(record: AdapterRecord, event: DiagnosticEvent, quantity: usize) {
    assert_eq!(
        record.availability,
        RecordAvailability::OffstagePhysicalMeasured
    );
    let tracking = event.reconstructed_reference().branches()[2].quantities[quantity];
    assert_bits(record.tracking_error.unwrap(), tracking.error.rms_error);
    let physical = event.reconstructed_physical().quantities()[quantity];
    let Evidence::Sequence(space) = record.channels[Channel::Space as usize] else {
        panic!("space missing")
    };
    let Evidence::Sequence(time) = record.channels[Channel::Time as usize] else {
        panic!("time missing")
    };
    let Evidence::Pair(method) = record.channels[Channel::Method as usize] else {
        panic!("method missing")
    };
    assert_eq!(
        space.map(f64::to_bits),
        [physical.pairs[0], physical.pairs[1]].map(|x| x.rms_error.to_bits())
    );
    assert_eq!(
        time.map(f64::to_bits),
        [physical.pairs[2], physical.pairs[3]].map(|x| x.rms_error.to_bits())
    );
    assert_bits(method, physical.pairs[4].rms_error);
    for channel in CHANNELS
        .into_iter()
        .filter(|channel| !matches!(channel, Channel::Space | Channel::Time | Channel::Method))
    {
        assert_missing(record.channels[channel as usize]);
    }
    let actual = record.reconstruction.unwrap().levels();
    let expected = event
        .residual()
        .sample()
        .unwrap()
        .temporal_geometry()
        .levels();
    for (left, right) in actual.into_iter().zip(expected) {
        assert_eq!((left.time(), left.nodes()), (right.time(), right.nodes()));
    }
}

fn assert_mapped(events: &[DiagnosticEvent], output: &[Option<AdapterRecord>; REVIEW_RECORDS]) {
    for (event_index, &event) in events.iter().enumerate() {
        for (quantity, observable) in OBSERVABLES.into_iter().enumerate() {
            let record = output[event_index * 4 + quantity].unwrap();
            assert_eq!(
                (record.clock, record.observable),
                (event.clock(), observable)
            );
            if event.accepted().sample().is_some() {
                assert_accepted(record, event, quantity);
            } else {
                assert_offstage(record, event, quantity);
            }
        }
    }
    let startup = output[0].unwrap();
    assert!(matches!(startup.tracking_error, Some(value) if value.to_bits() == 0));
    assert!(matches!(
        startup.channels[Channel::Space as usize],
        Evidence::Sequence([0.0, 0.0])
    ));
    assert!(matches!(
        startup.channels[Channel::Time as usize],
        Evidence::Sequence([0.0, 0.0])
    ));
    assert!(matches!(
        startup.channels[Channel::Method as usize],
        Evidence::Pair(0.0)
    ));
    assert!(!startup
        .channels
        .iter()
        .any(|item| matches!(item, Evidence::Floor { .. })));
}

fn main() {
    assert_eq!(
        OBSERVABLES.map(|item| item.quantity),
        [
            PhysicalQuantity::Vector,
            PhysicalQuantity::Gradient,
            PhysicalQuantity::Hessian,
            PhysicalQuantity::Vorticity
        ]
    );
    let accepted = clocks();
    let manifest = ticks(&[0, 7, 63, 64, 95, 127, 128]);
    let residual = ticks(&[7, 63, 95, 127]);
    let admitted = plan(&accepted, &manifest, &residual, 1e-5);
    let mut driver = DiagnosticDriver::new(admitted).unwrap();
    while driver.advance().unwrap().is_some() {}
    let events: [DiagnosticEvent; 7] = driver.reports().try_into().unwrap();
    let bounds = AdapterBounds::fixed();
    let mut output = [None; REVIEW_RECORDS];
    let allocation = Region::new(GLOBAL);
    let result = extract(
        admitted,
        profile(),
        &events,
        bounds.records,
        bounds.transactional_bytes,
        &mut output,
    )
    .unwrap();
    let spent = allocation.change();
    assert_eq!(
        (spent.allocations, spent.deallocations, spent.reallocations),
        (0, 0, 0)
    );
    assert_eq!(result.status, AdapterStatus::PartialUnqualifiedInventory);
    assert_eq!(result.case_sha256, CASE_SHA256);
    assert_eq!(result.profile.physical_samples, profile().physical_samples);
    assert_eq!(result.profile.tracking_samples, profile().tracking_samples);
    assert_eq!(
        result.profile.relative_floors.map(f64::to_bits),
        profile().relative_floors.map(f64::to_bits)
    );
    assert_eq!(result.records, REVIEW_RECORDS);
    assert_eq!(result.family_identity, admitted.family_plan().identity());
    assert_eq!(result.probe_identity, admitted.probe_plan().identity());
    assert_eq!(result.missing_channels.len(), 10);
    assert_eq!(result.missing_observable_groups.len(), 5);
    assert_mapped(&events, &output);

    let preserved = output[0].unwrap();
    assert_eq!(
        extract(
            admitted,
            profile(),
            &events[..6],
            bounds.records,
            bounds.transactional_bytes,
            &mut output
        ),
        Err(AdapterError::InvalidInput)
    );
    let mut reordered = events;
    reordered.swap(1, 2);
    assert_eq!(
        extract(
            admitted,
            profile(),
            &reordered,
            bounds.records,
            bounds.transactional_bytes,
            &mut output
        ),
        Err(AdapterError::InvalidInput)
    );
    let mut wrong_profile = profile();
    wrong_profile.relative_floors[0] = 2e-8;
    assert_eq!(
        extract(
            admitted,
            wrong_profile,
            &events,
            bounds.records,
            bounds.transactional_bytes,
            &mut output
        ),
        Err(AdapterError::InvalidInput)
    );
    assert_eq!(
        extract(
            admitted,
            profile(),
            &events,
            bounds.records - 1,
            bounds.transactional_bytes,
            &mut output
        ),
        Err(AdapterError::CapacityExceeded)
    );
    assert_eq!(
        extract(
            admitted,
            profile(),
            &events,
            bounds.records,
            bounds.transactional_bytes - 1,
            &mut output
        ),
        Err(AdapterError::CapacityExceeded)
    );
    let foreign = plan(&accepted, &manifest, &residual, 2e-5);
    assert_eq!(
        extract(
            foreign,
            profile(),
            &events,
            bounds.records,
            bounds.transactional_bytes,
            &mut output
        ),
        Err(AdapterError::InvalidInput)
    );
    assert_eq!(output[0].unwrap().clock, preserved.clock);
    assert_eq!(bounds.caller_output_bytes, std::mem::size_of_val(&output));
    println!(
        "v2 review extraction records={} caller_bytes={} transactional_bytes={} steady_allocations=0 status={:?}",
        result.records, bounds.caller_output_bytes, bounds.transactional_bytes, result.status
    );
}
