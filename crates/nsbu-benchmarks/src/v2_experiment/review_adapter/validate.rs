use super::{AdapterError, ReviewProfile, FINEST_CM_BRANCH, OBSERVABLES, REVIEW_RECORDS};
use crate::v2_experiment::diagnostic::{
    AcceptedSchedule, DiagnosticEvent, DiagnosticPlan, DiagnosticStatus, MissingChannel,
    ResidualSchedule, MISSING_CHANNELS,
};
use nsbu_solver::{diagnostics::physical::PhysicalQuantity, domain::TickClock};

pub(super) fn validate(
    plan: DiagnosticPlan<'_>,
    profile: ReviewProfile,
    events: &[DiagnosticEvent],
) -> Result<(), AdapterError> {
    let manifest = plan.probe_plan().tested_times().as_slice();
    require(
        events.len() == 7
            && events.len().checked_mul(OBSERVABLES.len()) == Some(REVIEW_RECORDS)
            && events
                .iter()
                .map(|event| event.clock())
                .eq(manifest.iter().copied()),
    )?;
    let family = plan.family_plan();
    let accepted_times = family.times().as_slice();
    for &event in events {
        validate_identity(event, family.identity(), plan.probe_plan().identity())?;
        validate_probe_physical(event, plan, profile)?;
        if accepted_times.contains(&event.clock()) {
            validate_accepted(
                event,
                profile,
                family.identity(),
                plan.probe_plan().identity(),
            )?;
        } else {
            validate_residual(event, plan.residual_times())?;
        }
    }
    Ok(())
}

fn validate_probe_physical(
    event: DiagnosticEvent,
    plan: DiagnosticPlan<'_>,
    profile: ReviewProfile,
) -> Result<(), AdapterError> {
    let sample = event.reconstructed_physical();
    let domains = std::array::from_fn(|index| {
        plan.family_plan()
            .branch_plan(index)
            .unwrap()
            .resources()
            .domain()
    });
    let quantities = OBSERVABLES.map(|item| item.quantity);
    require(
        sample.clock() == event.clock()
            && sample.identity() == event.probe_identity()
            && sample.reconstruction().clock() == event.probe().clock()
            && sample.reconstruction().identity() == event.probe().identity()
            && sample.origins() == event.probe().origins()
            && sample.source_domains() == domains
            && sample.sample_layout() == profile.physical_samples
            && sample.relative_floors().map(f64::to_bits)
                == profile.relative_floors.map(f64::to_bits)
            && sample.quantities().map(|item| item.quantity) == quantities,
    )
}

#[derive(PartialEq, Eq)]
struct EventIdentity {
    status: DiagnosticStatus,
    clock: TickClock,
    family: [u8; 32],
    probe: [u8; 32],
    probe_clock: TickClock,
    probe_sample: [u8; 32],
    missing: [MissingChannel; 10],
}

fn validate_identity(
    event: DiagnosticEvent,
    family: [u8; 32],
    probe: [u8; 32],
) -> Result<(), AdapterError> {
    let observed = EventIdentity {
        status: event.status(),
        clock: event.clock(),
        family: event.family_identity(),
        probe: event.probe_identity(),
        probe_clock: event.probe().clock(),
        probe_sample: event.probe().identity(),
        missing: *event.missing_channels(),
    };
    let expected = EventIdentity {
        status: DiagnosticStatus::UnqualifiedDiagnostic,
        clock: event.clock(),
        family,
        probe,
        probe_clock: event.clock(),
        probe_sample: probe,
        missing: MISSING_CHANNELS,
    };
    require(observed == expected)
}

#[derive(PartialEq, Eq)]
struct ResidualIdentity {
    schedules: (AcceptedSchedule, ResidualSchedule),
    accepted_absent: bool,
    clock: TickClock,
    reconstruction: [u8; 32],
    level_times: [TickClock; 3],
}

fn validate_residual(
    event: DiagnosticEvent,
    residual_times: &[TickClock],
) -> Result<(), AdapterError> {
    let sample = event
        .residual()
        .sample()
        .ok_or(AdapterError::InvalidInput)?;
    let observed = ResidualIdentity {
        schedules: (event.accepted().schedule(), event.residual().schedule()),
        accepted_absent: event.accepted().sample().is_none(),
        clock: sample.clock(),
        reconstruction: sample.reconstruction().identity(),
        level_times: sample
            .temporal_geometry()
            .levels()
            .map(|level| level.time()),
    };
    let expected = ResidualIdentity {
        schedules: (
            AcceptedSchedule::NotScheduledAtResidualClock,
            ResidualSchedule::Measured,
        ),
        accepted_absent: true,
        clock: event.clock(),
        reconstruction: event.probe_identity(),
        level_times: [event.clock(); 3],
    };
    require(residual_times.contains(&event.clock()) && observed == expected)
}

#[derive(PartialEq, Eq)]
struct AcceptedIdentity {
    schedules: (AcceptedSchedule, ResidualSchedule),
    residual_absent: bool,
    clocks: [TickClock; 5],
    families: [[u8; 32]; 5],
    probe: [u8; 32],
}

#[derive(PartialEq, Eq)]
struct AcceptedSemantics {
    layouts: [nsbu_solver::domain::Layout; 2],
    floors: [[u64; 4]; 2],
    branch: usize,
    quantities: [[PhysicalQuantity; 4]; 2],
}

fn validate_accepted(
    event: DiagnosticEvent,
    profile: ReviewProfile,
    family: [u8; 32],
    probe: [u8; 32],
) -> Result<(), AdapterError> {
    let sample = event
        .accepted()
        .sample()
        .ok_or(AdapterError::InvalidInput)?;
    let identity = AcceptedIdentity {
        schedules: (event.accepted().schedule(), event.residual().schedule()),
        residual_absent: event.residual().sample().is_none(),
        clocks: [
            sample.spectral.clock(),
            sample.physical.clock(),
            sample.pressure.clock(),
            sample.regional_reference.clock(),
            sample.node_binding.clock(),
        ],
        families: [
            sample.spectral.identity(),
            sample.physical.identity(),
            sample.pressure.identity(),
            sample.regional_reference.identity(),
            sample.node_binding.family_identity(),
        ],
        probe: sample.node_binding.probe_identity(),
    };
    let expected_identity = AcceptedIdentity {
        schedules: (
            AcceptedSchedule::Measured,
            ResidualSchedule::NotScheduledAtAcceptedClock,
        ),
        residual_absent: true,
        clocks: [event.clock(); 5],
        families: [family; 5],
        probe,
    };
    let branch = &sample.regional_reference.branches()[FINEST_CM_BRANCH];
    let semantics = AcceptedSemantics {
        layouts: [
            sample.physical.sample_layout(),
            sample.regional_reference.sample_layout(),
        ],
        floors: [
            sample.physical.relative_floors().map(f64::to_bits),
            sample
                .regional_reference
                .relative_floors()
                .map(f64::to_bits),
        ],
        branch: branch.branch,
        quantities: [
            sample.physical.quantities().map(|item| item.quantity),
            branch.quantities.map(|item| item.quantity),
        ],
    };
    let quantities = OBSERVABLES.map(|item| item.quantity);
    let expected_semantics = AcceptedSemantics {
        layouts: [profile.physical_samples, profile.tracking_samples],
        floors: [profile.relative_floors.map(f64::to_bits); 2],
        branch: FINEST_CM_BRANCH,
        quantities: [quantities; 2],
    };
    require(identity == expected_identity && semantics == expected_semantics)
}

fn require(valid: bool) -> Result<(), AdapterError> {
    if valid {
        Ok(())
    } else {
        Err(AdapterError::InvalidInput)
    }
}
