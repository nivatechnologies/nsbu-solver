//! Focused runtime force adapter contracts.
use nsbu_benchmarks::runtime_force::{ForceSettings, RunForce};
use nsbu_solver::{
    domain::{Domain, Layout, TickClock},
    integrators::forcing::PrescribedForce,
    Complex64, SolverError,
};

fn domain() -> Domain {
    Domain::new([4; 3], [1.0; 3], 1.0).unwrap()
}
fn output() -> [Vec<Complex64>; 3] {
    std::array::from_fn(|_| vec![Complex64::new(0.0, 0.0); domain().layout().half_len()])
}
fn bits(output: &[Vec<Complex64>; 3]) -> Vec<(u64, u64)> {
    output
        .iter()
        .flatten()
        .map(|v| (v.re.to_bits(), v.im.to_bits()))
        .collect()
}

#[test]
fn serial_and_parallel_match_coefficients_and_declared_work() {
    let samples = Layout::new([6; 3]).unwrap();
    let serial_settings = ForceSettings {
        samples,
        workers: 0,
    };
    let parallel_settings = ForceSettings {
        samples,
        workers: 2,
    };
    let serial_limits = serial_settings.limits(domain()).unwrap();
    let parallel_limits = parallel_settings.limits(domain()).unwrap();
    assert_eq!(serial_limits.work_units, parallel_limits.work_units);
    assert_eq!(
        serial_limits.scalar_transforms,
        parallel_limits.scalar_transforms
    );
    let mut serial = serial_settings
        .build(domain(), serial_limits.storage_bytes)
        .unwrap();
    let mut parallel = parallel_settings
        .build(domain(), parallel_limits.storage_bytes)
        .unwrap();
    assert_eq!(serial.limits(), Some(serial_limits));
    assert_eq!(parallel.limits(), Some(parallel_limits));
    for tick in [0, 1, 4, 2, 1, 0] {
        let clock = TickClock::restore(-10, 8, tick, 8 - tick).unwrap();
        let mut expected = output();
        let mut actual = output();
        let a = serial
            .evaluate(
                clock,
                serial_limits,
                expected.each_mut().map(Vec::as_mut_slice),
            )
            .unwrap();
        let b = parallel
            .evaluate(
                clock,
                parallel_limits,
                actual.each_mut().map(Vec::as_mut_slice),
            )
            .unwrap();
        assert_eq!(bits(&expected), bits(&actual));
        assert_eq!(
            (a.work_units, a.scalar_transforms),
            (b.work_units, b.scalar_transforms)
        );
    }
}

#[test]
fn settings_refuse_invalid_grid_workers_and_capacity() {
    let small = Layout::new([4; 3]).unwrap();
    assert!(ForceSettings {
        samples: small,
        workers: 0
    }
    .limits(domain())
    .is_ok());
    assert!(ForceSettings {
        samples: small,
        workers: 5
    }
    .limits(domain())
    .is_err());
    assert!(ForceSettings {
        samples: small,
        workers: 129
    }
    .limits(domain())
    .is_err());
    let limits = ForceSettings {
        samples: small,
        workers: 0,
    }
    .limits(domain())
    .unwrap();
    assert_eq!(
        ForceSettings {
            samples: small,
            workers: 0,
        }
        .build(domain(), limits.storage_bytes)
        .unwrap()
        .limits(),
        Some(limits)
    );
    assert!(matches!(
        ForceSettings {
            samples: small,
            workers: 0
        }
        .build(domain(), limits.storage_bytes - 1),
        Err(SolverError::ResourceLimit)
    ));
    assert!(ForceSettings {
        samples: small,
        workers: 0
    }
    .double_grid()
    .is_ok());
    let doubled = ForceSettings {
        samples: small,
        workers: 2,
    }
    .double_grid()
    .unwrap();
    assert_eq!(doubled.samples.dimensions(), [8; 3]);
    assert_eq!(doubled.workers, 2);
}

#[test]
fn run_force_has_no_qualification_claim() {
    let force = ForceSettings {
        samples: Layout::new([4; 3]).unwrap(),
        workers: 0,
    }
    .build(domain(), usize::MAX)
    .unwrap();
    assert!(matches!(force, RunForce::Serial(_)));
}
