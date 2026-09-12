use super::*;

#[test]
fn opt_in_w3_operator_matches_serial_and_refuses_one_byte_short() {
    let backend = FftBackend::RustFft6_4_1AvxFma;
    if backend.ensure_available().is_err() {
        return;
    }
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let catalog_bytes = FftCatalog::reservation(backend).unwrap();
    let catalog = FftCatalog::new(backend, catalog_bytes).unwrap();
    let serial_bytes = RotationalWorkspace::reservation_with_catalog(domain, &catalog).unwrap();
    let w3_bytes = RotationalWorkspace::reservation_with_w3_fft_backend(domain, backend).unwrap();
    let addition = W3FftPool::additional_reservation_with_backend(
        domain.padded_layout().unwrap(),
        backend,
        W3FftMode::Bidirectional,
    )
    .unwrap();
    assert_eq!(w3_bytes - serial_bytes, addition);
    assert!(matches!(
        RotationalWorkspace::new_with_catalog_w3(domain, &catalog, w3_bytes - 1),
        Err(SolverError::ResourceLimit)
    ));
    let mut serial = RotationalWorkspace::new_with_catalog(domain, &catalog, serial_bytes).unwrap();
    let mut w3 = RotationalWorkspace::new_with_catalog_w3(domain, &catalog, w3_bytes).unwrap();
    let h = domain.layout().half_len();
    let mut velocity = std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); h]);
    let mode = domain.layout().index([0, 0, 1]).unwrap();
    velocity[0][mode] = Complex64::new(0.25, -0.125);
    velocity[1][mode] = Complex64::new(-0.5, 0.375);
    let force = std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); h]);
    let mut serial_output = force.clone();
    let mut w3_output = force.clone();
    let mut serial_pressure = vec![Complex64::new(0.0, 0.0); h];
    let mut w3_pressure = serial_pressure.clone();
    serial
        .evaluate(
            velocity.each_ref().map(Vec::as_slice),
            force.each_ref().map(Vec::as_slice),
            serial_output.each_mut().map(Vec::as_mut_slice),
            &mut serial_pressure,
        )
        .unwrap();
    w3.evaluate(
        velocity.each_ref().map(Vec::as_slice),
        force.each_ref().map(Vec::as_slice),
        w3_output.each_mut().map(Vec::as_mut_slice),
        &mut w3_pressure,
    )
    .unwrap();
    assert_eq!(w3_output, serial_output);
    assert_eq!(w3_pressure, serial_pressure);
    assert_eq!(w3.w3_fft_identity().unwrap().additional_bytes, addition);
}

#[test]
fn drained_w3_failure_does_not_publish_rhs_or_pressure() {
    let backend = FftBackend::RustFft6_4_1AvxFma;
    if backend.ensure_available().is_err() {
        return;
    }
    let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
    let catalog_bytes = FftCatalog::reservation(backend).unwrap();
    let catalog = FftCatalog::new(backend, catalog_bytes).unwrap();
    let bytes = RotationalWorkspace::reservation_with_w3_fft_backend(domain, backend).unwrap();
    let mut owner = RotationalWorkspace::new_with_catalog_w3(domain, &catalog, bytes).unwrap();
    let h = domain.layout().half_len();
    let velocity = std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); h]);
    let force = velocity.clone();
    let sentinel = Complex64::new(17.0, -19.0);
    let mut output = std::array::from_fn(|_| vec![sentinel; h]);
    let mut pressure = vec![sentinel; h];
    let TransformOwner::W3(pool) = &owner.transform else {
        panic!("explicit W3 constructor returned a serial owner");
    };
    pool.inject_failure(1, false);
    assert_eq!(
        owner.evaluate(
            velocity.each_ref().map(Vec::as_slice),
            force.each_ref().map(Vec::as_slice),
            output.each_mut().map(Vec::as_mut_slice),
            &mut pressure,
        ),
        Err(SolverError::ArithmeticResolutionLimited)
    );
    assert!(output.iter().flatten().all(|value| *value == sentinel));
    assert!(pressure.iter().all(|value| *value == sentinel));
    let TransformOwner::W3(pool) = &owner.transform else {
        panic!("explicit W3 constructor returned a serial owner");
    };
    assert!(pool.is_terminated());
    assert!(pool.all_collected());
}
