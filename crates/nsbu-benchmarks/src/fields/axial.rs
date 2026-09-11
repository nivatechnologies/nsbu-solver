//! Per-call reuse of the identical axial implicit jet; this cache never contains velocity or state.
use super::{implicit_root, Evaluation};
use crate::{jet::Jet, root::RootReport, scalar, time::BenchmarkTime, BenchmarkError};
/// Checked primitive input identity and the unchanged formal-root outputs for one axial plane.
#[derive(Debug, Clone, Copy)]
pub(crate) struct AxialRoot {
    z: u64,
    time: [u64; 4],
    q: Jet,
    residual: Jet,
    report: RootReport,
}
impl AxialRoot {
    /// Every sampled plane includes x=y=0, so an interior plane's root is always used.
    /// Exact rest and exterior planes need no root or jet construction.
    pub(crate) fn new(z: f64, time: BenchmarkTime) -> Result<Option<Self>, BenchmarkError> {
        let z = scalar::periodic([0.0, 0.0, z])?[2];
        if time.elapsed() == 0.0 || z * z >= 441.0 / 2500.0 {
            return Ok(None);
        }
        let (q, residual, report) = implicit_root(Jet::variable(z, 2)?, time)?;
        Ok(Some(Self {
            z: z.to_bits(),
            time: key(time),
            q,
            residual,
            report,
        }))
    }
    /// Actual scalar iterations used once for this plane, not once for each reused field point.
    pub(crate) fn iterations(self) -> usize {
        self.report.iterations
    }
    fn bound(self, z: f64, time: BenchmarkTime) -> Result<(Jet, Jet, RootReport), BenchmarkError> {
        if self.z != z.to_bits() || self.time != key(time) {
            return Err(BenchmarkError::InvalidInput);
        }
        Ok((self.q, self.residual, self.report))
    }
}
fn key(time: BenchmarkTime) -> [u64; 4] {
    let errors = time.rounding_estimates();
    [
        time.elapsed().to_bits(),
        time.remaining().to_bits(),
        errors[0].to_bits(),
        errors[1].to_bits(),
    ]
}
/// Require the already prepared plane root for active points; never silently fall back or reuse stale data.
pub(crate) fn evaluate(
    point: [f64; 3],
    time: BenchmarkTime,
    root: Option<&AxialRoot>,
) -> Result<Evaluation, BenchmarkError> {
    let Some(point) = super::active_point(point, time)? else {
        return Ok(super::zero());
    };
    let root = root
        .ok_or(BenchmarkError::InvalidInput)?
        .bound(point[2], time)?;
    super::evaluate_active(point, time, root)
}

#[cfg(test)]
mod tests {
    use super::*;
    use nsbu_solver::domain::TickClock;
    fn time(t: u128) -> BenchmarkTime {
        BenchmarkTime::new(TickClock::restore(-10, 8, t, 8 - t).unwrap()).unwrap()
    }
    fn words(value: &Evaluation) -> Vec<u64> {
        let mut result = Vec::new();
        for row in [
            &value.velocity[..],
            &[value.pressure_raw],
            &value.force[..],
            &[value.divergence],
            &value.root_residual[..],
        ] {
            result.extend(row.iter().map(|v| v.to_bits()));
        }
        for row in value
            .force_gradient
            .into_iter()
            .chain(value.gradient_term_magnitudes)
        {
            result.extend(row.map(f64::to_bits));
        }
        for row in value.momentum_terms {
            result.extend(row.map(f64::to_bits));
        }
        if let Some(root) = value.root {
            result.extend(
                [
                    root.value,
                    root.bracket[0],
                    root.bracket[1],
                    root.residual,
                    root.error_estimate,
                ]
                .map(f64::to_bits),
            );
            result.push(root.iterations as u64);
        }
        result
    }
    #[test]
    fn every_force_derivative_and_root_word_matches_the_uncached_point_evaluator() {
        for tick in [1, 4, 2, 1, 0] {
            let time = time(tick);
            for z in [0.0, 0.125, 0.25, 0.375, 0.5, 0.75, 0.875] {
                let root = AxialRoot::new(z, time).unwrap();
                for (x, y) in [
                    (0.0, 0.0),
                    (0.125, 0.25),
                    (0.3, 0.1),
                    (0.4, 0.0),
                    (0.5, 0.5),
                ] {
                    let point = [x, y, z];
                    let full = super::super::evaluate(point, time).unwrap();
                    let cached = evaluate(point, time, root.as_ref()).unwrap();
                    assert_eq!(words(&full), words(&cached));
                    assert_eq!(full.root.is_some(), cached.root.is_some());
                }
            }
        }
    }
    #[test]
    fn missing_wrong_plane_and_stale_time_caches_are_refused_for_active_points() {
        let root = AxialRoot::new(0.125, time(1)).unwrap().unwrap();
        assert!(evaluate([0.0, 0.0, 0.125], time(1), None).is_err());
        assert!(evaluate([0.0; 3], time(1), Some(&root)).is_err());
        assert!(evaluate([0.0, 0.0, 0.125], time(2), Some(&root)).is_err());
        assert!(AxialRoot::new(f64::NAN, time(1)).is_err());
        assert!(evaluate([f64::NAN, 0.0, 0.0], time(1), Some(&root)).is_err());
        assert!(AxialRoot::new(0.5, time(1)).unwrap().is_none());
        assert!(AxialRoot::new(0.125, time(0)).unwrap().is_none());
        assert_eq!(
            evaluate([0.5, 0.5, 0.5], time(1), None).unwrap().force,
            [0.0; 3]
        );
    }
    #[test]
    fn remaining_coordinate_cannot_be_replaced_by_a_colliding_rounded_elapsed_time() {
        let target = 1u128 << 60;
        let a =
            BenchmarkTime::new(TickClock::restore(-67, target, target - 1, 1).unwrap()).unwrap();
        let b =
            BenchmarkTime::new(TickClock::restore(-67, target, target - 2, 2).unwrap()).unwrap();
        let root = AxialRoot::new(0.0, a).unwrap().unwrap();
        // Both elapsed conversions coincide, so binding only elapsed time would reuse the wrong root.
        assert_eq!(a.elapsed().to_bits(), b.elapsed().to_bits());
        assert_ne!(a.remaining().to_bits(), b.remaining().to_bits());
        assert!(evaluate([0.0; 3], b, Some(&root)).is_err());
    }
}
