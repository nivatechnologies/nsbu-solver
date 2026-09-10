//! Geometric masks never infer interior membership from a rounded cutoff value.
use super::INNER_RADIUS_SQUARED;
use crate::{
    root::{self, RootReport},
    scalar::periodic,
    time::BenchmarkTime,
    BenchmarkError,
};
use nsbu_solver::domain::TickClock;

/// Spatial masks use the declared spherical boundary and nominal similarity coordinates.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpatialRegion {
    /// Mathematical c_x=1, |eta|<=1/2 and X<=1/2.
    Core,
    /// Mathematical c_x=1, |eta|<=1/2 and 1/2<X<=8.
    Annulus,
    /// Mathematical c_x=1 outside the two mandatory nominal masks.
    InteriorOutsideNominal,
    /// Mathematical 0<c_x<1, even when binary64 evaluates c_x as exactly one or zero.
    Collar,
    /// Mathematical c_x=0 outside the cutoff support.
    Exterior,
}

/// Startup membership is decided with exact ticks, separately from the remaining coordinate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartupPhase {
    /// Exact initial time.
    Rest,
    /// Strictly between zero and 1/512.
    Ramping,
    /// At or after 1/512.
    Active,
}

/// Floating geometric classification; boundary-adjacent masks need an arithmetic refinement.
#[derive(Debug, Clone, Copy)]
pub struct PointRegion {
    /// Spatial class; no class replaces the global or collar error measurements.
    pub spatial: SpatialRegion,
    /// Exact startup phase.
    pub startup: StartupPhase,
    /// Root evidence only when nominal interior classification requires it.
    pub root: Option<RootReport>,
}

/// Classify a periodic point under the unchanged v2 definition with a finite root-work cap.
/// The spherical test uses r²<=9/100, never `smooth_step(...) == 1`.
pub fn classify(
    point: [f64; 3],
    clock: TickClock,
    root_budget: usize,
) -> Result<PointRegion, BenchmarkError> {
    if !(1..=128).contains(&root_budget) {
        return Err(BenchmarkError::InvalidInput);
    }
    let time = BenchmarkTime::new(clock)?;
    let [x, y, z] = periodic(point)?;
    let radius2 = x * x + y * y + z * z;
    let startup = if clock.elapsed() == 0 {
        StartupPhase::Rest
    } else if clock.elapsed() < clock.target() / 4 {
        StartupPhase::Ramping
    } else {
        StartupPhase::Active
    };
    let mut report = PointRegion {
        spatial: SpatialRegion::Exterior,
        startup,
        root: None,
    };
    if radius2 >= 441.0 / 2500.0 {
        return Ok(report);
    }
    if radius2 > INNER_RADIUS_SQUARED {
        report.spatial = SpatialRegion::Collar;
        return Ok(report);
    }
    let root = root::solve(z, time.remaining(), root_budget)?;
    let eta = z * root.value.powf(-3.0 / 8.0);
    let radial = (x * x + y * y) / (2.0 * root.value);
    report.spatial = if eta.abs() > 0.5 {
        SpatialRegion::InteriorOutsideNominal
    } else if radial <= 0.5 {
        SpatialRegion::Core
    } else if radial <= 8.0 {
        SpatialRegion::Annulus
    } else {
        SpatialRegion::InteriorOutsideNominal
    };
    report.root = Some(root);
    Ok(report)
}
