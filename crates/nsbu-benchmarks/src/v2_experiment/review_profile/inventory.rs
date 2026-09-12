//! Canonical exact-v2 observable semantics; values remain supplied by later consumers.
use sha2::{Digest, Sha256};

/// Number of required scalar observable rows at each fine-manifest clock.
pub const OBSERVABLE_COUNT: usize = 88;
/// First key in the dedicated complete-review inventory range.
pub const FIRST_KEY: u32 = 0x5632_1001;

/// Producer class that must supply an observable without substitution.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ObservableSource {
    /// Accepted-state complete fine-band spectral comparison.
    AcceptedSpectral = 1,
    /// Accepted-state physical comparison and analytical tracking.
    AcceptedPhysical = 2,
    /// Accepted-state pressure comparison and future pressure reference/gauge evidence.
    AcceptedPressure = 3,
    /// Accepted-state analytical tracking reduced on one named region.
    RegionalTracking = 4,
    /// Reconstructed off-stage momentum defect on its complete doubled band.
    OffStageResidual = 5,
    /// Accepted balance history with independently refined quadrature.
    BalanceQuadrature = 6,
}

/// Scalar quantity represented by one generic review row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ObservableQuantity {
    /// Three-component velocity.
    Velocity = 1,
    /// Nine ordered first spatial derivatives of velocity.
    Gradient = 2,
    /// Twenty-seven ordered second spatial derivatives of velocity.
    Hessian = 3,
    /// Three-component curl of velocity.
    Vorticity = 4,
    /// Zero-mean kinematic pressure.
    Pressure = 5,
    /// Three-component pressure gradient.
    PressureGradient = 6,
    /// Three-component momentum-equation defect.
    MomentumResidual = 7,
    /// Integrated kinetic-energy balance defect.
    EnergyDefect = 8,
    /// Integrated enstrophy balance defect.
    EnstrophyDefect = 9,
    /// Divergence of a vector field.
    Divergence = 10,
}

/// Exact scalar reduction applied to the typed quantity.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ObservableStatistic {
    /// Sample-average Euclidean or Frobenius error magnitude.
    SampledRmsError = 1,
    /// Largest sampled absolute error magnitude.
    SampledAbsolutePeakError = 2,
    /// Largest sampled error divided by the declared reference/floor scale.
    SampledRelativePeakError = 3,
    /// Difference after independently refined balance quadrature.
    QuadratureDefect = 4,
    /// Complete-band volume-average L2 norm.
    VolumeAverageL2 = 5,
    /// Complete-band nondimensional H1 norm including velocity and first derivatives.
    FourierH1 = 6,
}

/// Physical sample region; spectral and balance rows carry `NotRegional`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ObservableRegion {
    /// All points on the declared sample lattice.
    Global = 1,
    /// Sampled `c_x=1`, `0<=X<=1/2` class.
    Core = 2,
    /// Sampled `c_x=1`, `1/2<X<=8` class.
    Annulus = 3,
    /// Sampled interior points beyond the nominal core and annulus.
    InteriorOutsideNominal = 4,
    /// Sampled mathematical cutoff collar `0<c_x<1`.
    Collar = 5,
    /// Sampled points outside the cutoff support.
    Exterior = 6,
    /// Spectral or balance scalar without a spatial sample mask.
    NotRegional = 7,
}

/// Units of the scalar row; relative errors are explicitly dimensionless.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ObservableUnits {
    /// Velocity units.
    Velocity = 1,
    /// Velocity divided by length.
    VelocityPerLength = 2,
    /// Velocity divided by squared length.
    VelocityPerLengthSquared = 3,
    /// Kinematic-pressure units.
    Pressure = 4,
    /// Kinematic pressure divided by length.
    PressurePerLength = 5,
    /// Unitless relative error.
    Dimensionless = 6,
    /// Momentum residual, or acceleration, units.
    Acceleration = 7,
    /// Nondimensional combined H1 momentum-residual units.
    AccelerationH1 = 8,
    /// Kinetic-energy units.
    Energy = 9,
    /// Enstrophy units.
    Enstrophy = 10,
    /// Nondimensional combined velocity H1 units.
    VelocityH1 = 11,
}

/// Complete typed meaning of one stable generic-protocol key.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ObservableDescriptor {
    /// Stable generic-protocol key.
    pub key: u32,
    /// Required producer class.
    pub source: ObservableSource,
    /// Complete scalar/vector/tensor subject before reduction.
    pub quantity: ObservableQuantity,
    /// Defined scalar reduction.
    pub statistic: ObservableStatistic,
    /// Physical mask, or explicit absence of one.
    pub region: ObservableRegion,
    /// Units of the resulting scalar.
    pub units: ObservableUnits,
}

const EMPTY: ObservableDescriptor = ObservableDescriptor {
    key: 0,
    source: ObservableSource::AcceptedSpectral,
    quantity: ObservableQuantity::Velocity,
    statistic: ObservableStatistic::SampledRmsError,
    region: ObservableRegion::Global,
    units: ObservableUnits::Velocity,
};

/// Frozen complete inventory in generic review order.
pub const OBSERVABLES: [ObservableDescriptor; OBSERVABLE_COUNT] = build_inventory();

const fn build_inventory() -> [ObservableDescriptor; OBSERVABLE_COUNT] {
    let mut out = [EMPTY; OBSERVABLE_COUNT];
    let mut cursor = 0;
    let fields = [
        ObservableQuantity::Velocity,
        ObservableQuantity::Gradient,
        ObservableQuantity::Hessian,
        ObservableQuantity::Vorticity,
    ];
    let regions = [
        ObservableRegion::Core,
        ObservableRegion::Annulus,
        ObservableRegion::InteriorOutsideNominal,
        ObservableRegion::Collar,
        ObservableRegion::Exterior,
    ];
    let statistics = [
        ObservableStatistic::SampledRmsError,
        ObservableStatistic::SampledAbsolutePeakError,
        ObservableStatistic::SampledRelativePeakError,
    ];
    out[cursor] = norm(
        cursor,
        ObservableSource::AcceptedSpectral,
        ObservableQuantity::Velocity,
        ObservableStatistic::VolumeAverageL2,
        ObservableUnits::Velocity,
    );
    cursor += 1;
    out[cursor] = norm(
        cursor,
        ObservableSource::AcceptedSpectral,
        ObservableQuantity::Velocity,
        ObservableStatistic::FourierH1,
        ObservableUnits::VelocityH1,
    );
    cursor += 1;
    out[cursor] = norm(
        cursor,
        ObservableSource::AcceptedSpectral,
        ObservableQuantity::Vorticity,
        ObservableStatistic::VolumeAverageL2,
        ObservableUnits::VelocityPerLength,
    );
    cursor += 1;
    out[cursor] = norm(
        cursor,
        ObservableSource::AcceptedSpectral,
        ObservableQuantity::Divergence,
        ObservableStatistic::VolumeAverageL2,
        ObservableUnits::VelocityPerLength,
    );
    cursor += 1;
    let mut field = 0;
    while field < fields.len() {
        let mut statistic = 0;
        while statistic < statistics.len() {
            out[cursor] = sampled(
                cursor,
                ObservableSource::AcceptedPhysical,
                fields[field],
                statistics[statistic],
                ObservableRegion::Global,
            );
            cursor += 1;
            statistic += 1;
        }
        field += 1;
    }
    let pressure = [
        ObservableQuantity::Pressure,
        ObservableQuantity::PressureGradient,
    ];
    let mut quantity = 0;
    while quantity < pressure.len() {
        let mut statistic = 0;
        while statistic < statistics.len() {
            out[cursor] = sampled(
                cursor,
                ObservableSource::AcceptedPressure,
                pressure[quantity],
                statistics[statistic],
                ObservableRegion::Global,
            );
            cursor += 1;
            statistic += 1;
        }
        quantity += 1;
    }
    let mut region = 0;
    while region < regions.len() {
        let mut field = 0;
        while field < fields.len() {
            let mut statistic = 0;
            while statistic < statistics.len() {
                out[cursor] = sampled(
                    cursor,
                    ObservableSource::RegionalTracking,
                    fields[field],
                    statistics[statistic],
                    regions[region],
                );
                cursor += 1;
                statistic += 1;
            }
            field += 1;
        }
        region += 1;
    }
    out[cursor] = norm(
        cursor,
        ObservableSource::OffStageResidual,
        ObservableQuantity::MomentumResidual,
        ObservableStatistic::VolumeAverageL2,
        ObservableUnits::Acceleration,
    );
    cursor += 1;
    out[cursor] = norm(
        cursor,
        ObservableSource::OffStageResidual,
        ObservableQuantity::MomentumResidual,
        ObservableStatistic::FourierH1,
        ObservableUnits::AccelerationH1,
    );
    cursor += 1;
    out[cursor] = norm(
        cursor,
        ObservableSource::OffStageResidual,
        ObservableQuantity::Vorticity,
        ObservableStatistic::VolumeAverageL2,
        ObservableUnits::AccelerationH1,
    );
    cursor += 1;
    out[cursor] = norm(
        cursor,
        ObservableSource::OffStageResidual,
        ObservableQuantity::Divergence,
        ObservableStatistic::VolumeAverageL2,
        ObservableUnits::AccelerationH1,
    );
    cursor += 1;
    out[cursor] = balance(
        cursor,
        ObservableQuantity::EnergyDefect,
        ObservableUnits::Energy,
    );
    cursor += 1;
    out[cursor] = balance(
        cursor,
        ObservableQuantity::EnstrophyDefect,
        ObservableUnits::Enstrophy,
    );
    out
}

const fn sampled(
    index: usize,
    source: ObservableSource,
    quantity: ObservableQuantity,
    statistic: ObservableStatistic,
    region: ObservableRegion,
) -> ObservableDescriptor {
    ObservableDescriptor {
        key: FIRST_KEY + index as u32,
        source,
        quantity,
        statistic,
        region,
        units: sampled_units(quantity, statistic),
    }
}
const fn sampled_units(
    quantity: ObservableQuantity,
    statistic: ObservableStatistic,
) -> ObservableUnits {
    if matches!(statistic, ObservableStatistic::SampledRelativePeakError) {
        return ObservableUnits::Dimensionless;
    }
    match quantity {
        ObservableQuantity::Velocity => ObservableUnits::Velocity,
        ObservableQuantity::Gradient | ObservableQuantity::Vorticity => {
            ObservableUnits::VelocityPerLength
        }
        ObservableQuantity::Hessian => ObservableUnits::VelocityPerLengthSquared,
        ObservableQuantity::Pressure => ObservableUnits::Pressure,
        ObservableQuantity::PressureGradient => ObservableUnits::PressurePerLength,
        _ => ObservableUnits::Dimensionless,
    }
}
const fn norm(
    index: usize,
    source: ObservableSource,
    quantity: ObservableQuantity,
    statistic: ObservableStatistic,
    units: ObservableUnits,
) -> ObservableDescriptor {
    ObservableDescriptor {
        key: FIRST_KEY + index as u32,
        source,
        quantity,
        statistic,
        region: ObservableRegion::NotRegional,
        units,
    }
}
const fn balance(
    index: usize,
    quantity: ObservableQuantity,
    units: ObservableUnits,
) -> ObservableDescriptor {
    ObservableDescriptor {
        key: FIRST_KEY + index as u32,
        source: ObservableSource::BalanceQuadrature,
        quantity,
        statistic: ObservableStatistic::QuadratureDefect,
        region: ObservableRegion::NotRegional,
        units,
    }
}

/// Mandatory semantics that currently lack a scalar producer or reviewed reduction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum MandatoryObservableGap {
    /// Analytical pressure comparison is not produced.
    PressureReference = 1,
    /// Zero-mean/reference gauge equivalence is not produced.
    PressureGauge = 2,
    /// Actual versus reference concentration peak-height discrepancy lacks a producer.
    PeakHeight = 3,
    /// Periodic peak-location distance and tie rule lack a producer.
    PeakLocation = 4,
    /// Nominal core volume-intersection refinement is outside sampled region counts.
    CoreNominalCoverage = 5,
    /// Nominal annulus volume-intersection refinement is outside sampled region counts.
    AnnulusNominalCoverage = 6,
    /// Cutoff-collar volume coverage lacks a reviewed consumer.
    CollarVolumeCoverage = 7,
    /// Sampled rows do not establish full interior/global qualification.
    CompleteGlobalQualification = 8,
    /// Mean velocity versus mean force balance lacks a bound consumer.
    MeanMomentumBalance = 9,
}
/// Gaps are part of semantics identity and may never be represented by zero rows.
pub const MANDATORY_GAPS: [MandatoryObservableGap; 9] = [
    MandatoryObservableGap::PressureReference,
    MandatoryObservableGap::PressureGauge,
    MandatoryObservableGap::PeakHeight,
    MandatoryObservableGap::PeakLocation,
    MandatoryObservableGap::CoreNominalCoverage,
    MandatoryObservableGap::AnnulusNominalCoverage,
    MandatoryObservableGap::CollarVolumeCoverage,
    MandatoryObservableGap::CompleteGlobalQualification,
    MandatoryObservableGap::MeanMomentumBalance,
];

/// SHA-256 over the complete versioned typed inventory and explicit gaps, independent of budgets.
pub fn semantics_identity() -> [u8; 32] {
    let mut hash = Sha256::new();
    hash.update(b"NSBUV2OBSERVABLES0001");
    hash.update((OBSERVABLE_COUNT as u128).to_le_bytes());
    for item in OBSERVABLES {
        hash.update(item.key.to_le_bytes());
        hash.update([
            item.source as u8,
            item.quantity as u8,
            item.statistic as u8,
            item.region as u8,
            item.units as u8,
        ]);
    }
    hash.update((MANDATORY_GAPS.len() as u128).to_le_bytes());
    for gap in MANDATORY_GAPS {
        hash.update([gap as u8]);
    }
    hash.finalize().into()
}
