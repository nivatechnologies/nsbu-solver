use super::*;
use crate::{
    domain::{Epoch, ExtraStorage, TickClock},
    integrators::{
        coefficients::CmCoefficients, ho_coefficients::HoCoefficients, kernel::RhsBounds,
    },
    Complex64,
};

fn domain() -> Domain {
    Domain::new([4; 3], [1.0, 2.0, 3.0], 0.5).unwrap()
}

fn plan(method: Method) -> ResourcePlan {
    let domain = domain();
    ResourcePlan::new(
        domain,
        ExtraStorage {
            fft: 0,
            force: 0,
            diagnostics: AttemptWorkspace::reservation_with_method(domain, method).unwrap(),
            overhead: 4096,
        },
        2 * 1024 * 1024,
        Epoch(0),
    )
    .unwrap()
}

fn workspace(method: Method) -> AttemptWorkspace {
    AttemptWorkspace::new_with_method(plan(method), method).unwrap()
}

fn cm_words(value: &CmCoefficients) -> [u64; 7] {
    [
        value.exponential,
        value.half_exponential,
        value.q,
        value.weights[0],
        value.weights[1],
        value.weights[2],
        value.truncation_bound,
    ]
    .map(f64::to_bits)
}

fn ho_words(value: &HoCoefficients) -> Vec<u64> {
    [value.exponential, value.half_exponential]
        .into_iter()
        .chain(value.rows.into_iter().flatten())
        .chain(value.weights)
        .chain([value.truncation_bound])
        .map(f64::to_bits)
        .collect()
}

fn words(workspace: &AttemptWorkspace) -> Vec<u64> {
    match &workspace.kernel {
        MethodWorkspace::Cm(_, full, half) => full.iter().chain(half).flat_map(cm_words).collect(),
        MethodWorkspace::Ho(_, full, half) => full.iter().chain(half).flat_map(ho_words).collect(),
    }
}

#[test]
fn exact_hit_and_forced_miss_match_every_cm_and_ho_table_word() {
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        let mut cached = workspace(method);
        cached.coefficients(0.125, 0.0625).unwrap();
        let expected = words(&cached);
        cached.coefficients(0.125, 0.0625).unwrap();
        assert_eq!(words(&cached), expected);

        let mut forced_miss = workspace(method);
        forced_miss.coefficients(0.25, 0.125).unwrap();
        forced_miss.coefficients(0.125, 0.0625).unwrap();
        assert_eq!(words(&forced_miss), expected);
        assert_eq!(forced_miss.coefficients, cached.coefficients);
    }
}

#[test]
fn failed_partial_rebuild_invalidates_then_repairs_every_word() {
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        let mut cached = workspace(method);
        cached.coefficients(0.125, 0.0625).unwrap();
        let expected = words(&cached);
        assert_eq!(
            cached.coefficients(f64::MAX, f64::MAX / 2.0),
            Err(SolverError::ArithmeticResolutionLimited)
        );
        assert!(!cached.coefficients.is_valid());
        cached.coefficients(0.125, 0.0625).unwrap();
        assert_eq!(words(&cached), expected);
    }
}

struct FailingRhs;
impl RightHandSide for FailingRhs {
    fn bounds(&self) -> Option<RhsBounds> {
        Some(RhsBounds {
            storage_bytes: 0,
            work_units: 1,
            scalar_transforms: 0,
        })
    }
    fn evaluate(
        &mut self,
        _state: [&[Complex64]; 3],
        _time: TickClock,
        _output: [&mut [Complex64]; 3],
    ) -> Result<(), SolverError> {
        Err(SolverError::InvalidSpectrum)
    }
}

#[test]
fn downstream_rhs_failure_retains_completed_tables_and_key() {
    for method in [Method::CoxMatthews, Method::HochbruckOstermann] {
        let plan = plan(method);
        let clock = TickClock::from_rest(-5, 64).unwrap();
        let state = SpectralState::from_rest(plan, clock, Epoch(0)).unwrap();
        let mut candidate = CandidateState::new(plan, clock, Epoch(0)).unwrap();
        let mut workspace = AttemptWorkspace::new_with_method(plan, method).unwrap();
        workspace.coefficients(0.125, 0.0625).unwrap();
        let key = workspace.coefficients;
        let expected = words(&workspace);
        assert_eq!(
            workspace
                .try_advance(
                    &state,
                    &mut candidate,
                    4,
                    Tolerances {
                        absolute: [1.0; 2],
                        relative: [0.0; 2],
                    },
                    &mut FailingRhs,
                )
                .unwrap_err(),
            SolverError::InvalidSpectrum
        );
        assert_eq!(workspace.coefficients, key);
        assert_eq!(words(&workspace), expected);
    }
}

#[test]
fn cache_key_and_default_workspace_resource_abi_are_exact() {
    assert_eq!(std::mem::size_of::<CoefficientKey>(), 16);
    assert_eq!(std::mem::size_of::<AttemptWorkspace>(), 800);
}
