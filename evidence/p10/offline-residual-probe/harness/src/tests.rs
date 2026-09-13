use super::*;
use crate::model::{Claims, NodeBinding, ProbePlan};
use nsbu_solver::integrators::forcing::{ForceLimits, ForceWork};
use std::path::PathBuf;

fn fixture_plan() -> ProbePlan {
    let identity = format!(
        "source={SOURCE};case={CASE_SHA256};profile={PROFILE};retained=384;force_samples=384;observer_force_samples=768;method=cox-matthews"
    );
    ProbePlan {
        schema: "p10-offline-residual-probe-plan-v1".into(),
        status: "frozen_one_probe_not_executed".into(),
        source_commit: SOURCE.into(),
        w3_source_commit: W3_SOURCE.into(),
        case_sha256: CASE_SHA256.into(),
        frozen_plan: "plan.json".into(),
        frozen_plan_sha256: PLAN_SHA256.into(),
        profile: PROFILE.into(),
        snapshot_identity: identity,
        dimensions: [384; 3],
        lengths: [1.0; 3],
        viscosity: 1.0,
        quantum_exponent: -20,
        clock_target: 8192,
        method: "cox-matthews".into(),
        integration_force_dimensions: [384; 3],
        integration_force_workers: 32,
        residual_force_dimensions: [768; 3],
        residual_force_workers: 32,
        advective_limit: 3.3,
        probe_clock: PROBE,
        supports: SUPPORTS,
        nodes: [896, 1024, 1088, 1152, 1216, 1280, 1408]
            .into_iter()
            .map(|clock| NodeBinding {
                clock,
                epoch: clock / 64,
                accepted_steps: clock / 64,
                snapshot: PathBuf::from(format!("clock-{clock}/state.bin")),
                coefficient_sha256: "a".repeat(64),
                file_sha256: "b".repeat(64),
            })
            .collect(),
        claims: Claims {
            runtime_owner_imported: false,
            accepted_interpolation: false,
            acceptance_windows: 0,
            arithmetic: "binary64".into(),
            purpose: "read-only-offline-diagnostic".into(),
        },
    }
}

#[test]
fn exact_lineage_and_one_probe_shape_are_enforced() {
    let mut plan = fixture_plan();
    validate_plan(&plan).unwrap();
    plan.source_commit.replace_range(..1, "b");
    assert!(validate_plan(&plan).unwrap_err().contains("lineage"));
    plan = fixture_plan();
    plan.supports[1][0] += 1;
    assert!(validate_plan(&plan).unwrap_err().contains("lineage"));
    plan = fixture_plan();
    plan.claims.accepted_interpolation = true;
    assert!(validate_plan(&plan).unwrap_err().contains("lineage"));
    plan = fixture_plan();
    plan.nodes[0].file_sha256 = "not-a-hash".into();
    assert!(validate_plan(&plan).unwrap_err().contains("node binding"));
}

#[derive(Clone, Copy)]
struct ZeroForce;

impl PrescribedForce for ZeroForce {
    fn limits(&self) -> Option<ForceLimits> {
        Some(ForceLimits {
            storage_bytes: std::mem::size_of::<Self>(),
            work_units: 1,
            scalar_transforms: 0,
            remaining_divisor: 1,
        })
    }

    fn evaluate(
        &mut self,
        _time: TickClock,
        limit: ForceLimits,
        output: [&mut [Complex64]; 3],
    ) -> Result<ForceWork, SolverError> {
        if Some(limit) != self.limits() {
            return Err(SolverError::ProviderBudgetExceeded);
        }
        for component in output {
            component.fill(Complex64::new(0.0, 0.0));
        }
        Ok(ForceWork {
            work_units: 1,
            scalar_transforms: 0,
        })
    }
}

#[test]
fn explicit_rhs_plus_one_viscous_term_has_the_residual_plan_sign() {
    let source = Domain::new([4; 3], [1.0; 3], 0.25).unwrap();
    let mut velocity = field(source).unwrap();
    let positive = source.layout().index([1, 0, 0]).unwrap();
    let negative = source.layout().index([3, 0, 0]).unwrap();
    velocity[1][positive] = Complex64::new(0.125, -0.25);
    velocity[1][negative] = velocity[1][positive].conj();

    let limits = ZeroForce.limits().unwrap();
    let cap = SpectralRhs::<ZeroForce>::reservation(source, limits).unwrap();
    let mut rhs = SpectralRhs::new(source, ZeroForce, 10.0, cap).unwrap();
    let start = TickClock::restore(-20, 8192, 64, 8128).unwrap();
    let mut derivative = field(source).unwrap();
    rhs.begin_attempt(start, 1).unwrap();
    rhs.evaluate(
        velocity.each_ref().map(Vec::as_slice),
        start,
        derivative.each_mut().map(Vec::as_mut_slice),
    )
    .unwrap();
    assert!(derivative
        .iter()
        .flatten()
        .all(|value| value.norm() < 1e-12));

    add_viscosity(source, &velocity, &mut derivative).unwrap();
    let k = 2.0 * std::f64::consts::PI;
    let expected = -source.viscosity() * k * k * velocity[1][positive];
    assert!((derivative[1][positive] - expected).norm() < 1e-13);

    let diagnostic = ConservativeWorkspace::diagnostic_domain(source).unwrap();
    let conservative = field(diagnostic).unwrap();
    let mut residual = field(diagnostic).unwrap();
    let norms = ResidualPlan::new(source)
        .unwrap()
        .evaluate(
            velocity.each_ref().map(Vec::as_slice),
            derivative.each_ref().map(Vec::as_slice),
            conservative.each_ref().map(Vec::as_slice),
            residual.each_mut().map(Vec::as_mut_slice),
        )
        .unwrap();
    assert!(norms.l2 < 1e-12);
    assert!(norms.h1 < 1e-11);
    assert!(norms.vorticity_l2 < 1e-11);
    assert!(norms.divergence_l2 < 1e-12);
}

#[test]
fn localized_force_delta_has_plus_sign_and_strict_shell_mask() {
    let source = Domain::new([4; 3], [1.0; 3], 0.25).unwrap();
    let diagnostic = ConservativeWorkspace::diagnostic_domain(source).unwrap();
    let velocity = field(source).unwrap();
    let derivative = field(source).unwrap();
    let mut retained_force = field(source).unwrap();
    let mut full_force = field(diagnostic).unwrap();
    let mut conservative = field(diagnostic).unwrap();
    let retained_positive = source.layout().index([1, 0, 0]).unwrap();
    let retained_negative = source.layout().index([3, 0, 0]).unwrap();
    retained_force[1][retained_positive] = Complex64::new(0.25, 0.0);
    retained_force[1][retained_negative] = Complex64::new(0.25, 0.0);
    let full_retained_positive = diagnostic.layout().index([1, 0, 0]).unwrap();
    let full_retained_negative = diagnostic.layout().index([7, 0, 0]).unwrap();
    let shell_positive = diagnostic.layout().index([3, 0, 0]).unwrap();
    let shell_negative = diagnostic.layout().index([5, 0, 0]).unwrap();
    for index in [full_retained_positive, full_retained_negative] {
        full_force[1][index] = Complex64::new(1.0, 0.0);
        conservative[1][index] = Complex64::new(-1.0, 0.0);
    }
    for index in [shell_positive, shell_negative] {
        full_force[1][index] = Complex64::new(2.0, 0.0);
        conservative[1][index] = Complex64::new(-2.0, 0.0);
    }
    let mut ordinary = field(diagnostic).unwrap();
    let ordinary_norms = ResidualPlan::new(source)
        .unwrap()
        .evaluate(
            velocity.each_ref().map(Vec::as_slice),
            derivative.each_ref().map(Vec::as_slice),
            conservative.each_ref().map(Vec::as_slice),
            ordinary.each_mut().map(Vec::as_mut_slice),
        )
        .unwrap();
    let mut localized = field(diagnostic).unwrap();
    let result = ResidualPlan::new(source)
        .unwrap()
        .evaluate_localized(
            velocity.each_ref().map(Vec::as_slice),
            derivative.each_ref().map(Vec::as_slice),
            conservative.each_ref().map(Vec::as_slice),
            full_force.each_ref().map(Vec::as_slice),
            retained_force.each_ref().map(Vec::as_slice),
            localized.each_mut().map(Vec::as_mut_slice),
        )
        .unwrap();
    assert_eq!(ordinary, localized);
    assert!((ordinary_norms.l2 - result.full_n768.residual_m768.l2).abs() < 1e-14);
    assert!((ordinary_norms.h1 - result.full_n768.residual_m768.h1).abs() < 1e-13);
    assert!(
        (ordinary_norms.vorticity_l2 - result.full_n768.residual_m768.vorticity_l2).abs() < 1e-13
    );
    assert_eq!(
        ordinary_norms.divergence_l2,
        result.full_n768.residual_m768.divergence_l2
    );
    assert_eq!(result.retained_modes, 18);
    assert_eq!(result.new_shell_modes, 178);
    assert_eq!(result.excluded_nyquist_slots, 124);
    assert_eq!(
        localized[1][shell_positive],
        conservative[1][shell_positive]
    );
    assert!((result.full_n768.residual_m384.l2 - 0.25 * 2.0_f64.sqrt()).abs() < 1e-13);
    enforce_identity_tolerance(&result, 1e-12).unwrap();
}

#[test]
fn selected_retained_force_constructor_matches_scalar_avx_reference() {
    let backend = FftBackend::RustFft6_4_1AvxFma;
    if backend.ensure_available().is_err() {
        return;
    }
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let samples = Layout::new([6; 3]).unwrap();
    let catalog_bytes = FftCatalog::reservation(backend).unwrap();
    let catalog = FftCatalog::new(backend, catalog_bytes).unwrap();
    let selected_limits = CachedReducedForce::preflight(domain, samples, 3, backend, true).unwrap();
    let scalar_limits = CachedReducedForce::preflight(domain, samples, 3, backend, false).unwrap();
    let mut selected = CachedReducedForce::new(
        domain,
        samples,
        3,
        &catalog,
        selected_limits.storage_bytes,
        true,
    )
    .unwrap();
    let mut scalar = CachedReducedForce::new(
        domain,
        samples,
        3,
        &catalog,
        scalar_limits.storage_bytes,
        false,
    )
    .unwrap();
    let identity = selected.w3_identity().unwrap();
    assert_eq!(identity.backend, backend);
    assert_eq!(identity.layout, samples);
    assert_eq!(identity.width, 3);
    assert_eq!(identity.mode, W3FftMode::Forward);
    let time = TickClock::restore(-20, 8192, PROBE, 8192 - PROBE).unwrap();
    let mut actual = field(domain).unwrap();
    let mut expected = field(domain).unwrap();
    selected.begin_attempt(time, 64, selected_limits).unwrap();
    scalar.begin_attempt(time, 64, scalar_limits).unwrap();
    selected
        .evaluate(
            time,
            selected_limits,
            actual.each_mut().map(Vec::as_mut_slice),
        )
        .unwrap();
    scalar
        .evaluate(
            time,
            scalar_limits,
            expected.each_mut().map(Vec::as_mut_slice),
        )
        .unwrap();
    assert_eq!(actual, expected);
    assert_eq!(selected.hit_miss(), [0, 1]);
    assert_eq!(scalar.hit_miss(), [0, 1]);
}

#[test]
fn selected_w3_constructor_matches_the_archived_scalar_avx_reference_bits() {
    let backend = FftBackend::RustFft6_4_1AvxFma;
    if backend.ensure_available().is_err() {
        return;
    }
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let samples = Layout::new([6; 3]).unwrap();
    let mut selected = build_w3_rhs(domain, samples, 3, 10.0).unwrap();
    let mut archived_scalar = build_scalar_rhs_reference(domain, samples, 3, 10.0).unwrap();
    let rhs_identity = selected.w3_fft_identity().unwrap();
    let force_identity = selected.provider().w3_identity().unwrap();
    assert_eq!(rhs_identity.backend, backend);
    assert_eq!(rhs_identity.width, 3);
    assert_eq!(rhs_identity.mode, W3FftMode::Bidirectional);
    assert_eq!(rhs_identity.layout, domain.padded_layout().unwrap());
    assert_eq!(force_identity.backend, backend);
    assert_eq!(force_identity.width, 3);
    assert_eq!(force_identity.mode, W3FftMode::Forward);
    assert_eq!(force_identity.layout, samples);

    let mut velocity = field(domain).unwrap();
    let positive = domain.layout().index([1, 0, 0]).unwrap();
    let negative = domain.layout().index([3, 0, 0]).unwrap();
    velocity[1][positive] = Complex64::new(0.125, -0.25);
    velocity[1][negative] = velocity[1][positive].conj();
    let time = TickClock::restore(-20, 8192, 64, 8128).unwrap();
    let mut actual = field(domain).unwrap();
    let mut reference = field(domain).unwrap();
    selected.begin_attempt(time, 64).unwrap();
    archived_scalar.begin_attempt(time, 64).unwrap();
    selected
        .evaluate(
            velocity.each_ref().map(Vec::as_slice),
            time,
            actual.each_mut().map(Vec::as_mut_slice),
        )
        .unwrap();
    archived_scalar
        .evaluate(
            velocity.each_ref().map(Vec::as_slice),
            time,
            reference.each_mut().map(Vec::as_mut_slice),
        )
        .unwrap();
    assert_eq!(actual, reference);
    assert_eq!(selected.provider().hit_miss(), [0, 1]);
    assert_eq!(archived_scalar.provider().hit_miss(), [0, 1]);
    add_viscosity(domain, &velocity, &mut actual).unwrap();
    add_viscosity(domain, &velocity, &mut reference).unwrap();
    assert_eq!(actual, reference);
}

#[test]
fn hermite_kernel_reproduces_a_binary64_linear_signal() {
    let support = SUPPORTS[1];
    let weights = HermiteWeights::at(support.map(clock), clock(PROBE)).unwrap();
    let scale = 2.0_f64.powi(-20);
    let values: [Vec<Complex64>; 3] =
        support.map(|tick| vec![Complex64::new(3.0 + 7.0 * tick as f64 * scale, 0.0)]);
    let derivatives: [Vec<Complex64>; 3] = std::array::from_fn(|_| vec![Complex64::new(7.0, 0.0)]);
    let mut value = [Complex64::new(0.0, 0.0)];
    let mut derivative = [Complex64::new(0.0, 0.0)];
    weights
        .apply(
            [
                &values[0],
                &values[1],
                &values[2],
                &derivatives[0],
                &derivatives[1],
                &derivatives[2],
            ],
            &mut value,
            &mut derivative,
        )
        .unwrap();
    let expected = 3.0 + 7.0 * PROBE as f64 * scale;
    assert!((value[0].re - expected).abs() < 1e-14);
    assert!((derivative[0].re - 7.0).abs() < 1e-12);
}
