//! Immutable endpoint profile and its complete memory/work/disk admission.

mod admission;
mod identity;
mod profile;

pub use admission::{
    domain, preflight, require_execution_ready, require_run_authorized, tolerances,
};
pub use identity::identity;
pub use profile::{ADVECTIVE_LIMIT, CAP, M, N, OBSERVER_M, WORKERS};
#[cfg(feature = "n512-m512-parallel-capture")]
pub use profile::FFT_WORKERS;
#[cfg(all(test, feature = "n512-m512-piecewise-cadv33"))]
pub(crate) use admission::require_n512_gate;
#[cfg(all(
    test,
    any(
        feature = "n512-m512-temporal-h32",
        feature = "n512-m512-temporal-h16"
    )
))]
pub(crate) use admission::require_temporal_gate;
#[cfg(all(test, feature = "n256-m512-piecewise-cadv33"))]
pub(crate) use admission::require_n256_gate;
#[cfg(all(
    test,
    any(
        feature = "n512-m512-temporal-h32",
        feature = "n512-m512-temporal-h16"
    )
))]
pub(crate) use admission::{
    admit, diagnostic_reservations, disk_bound, geometry, resources, reservations, work_admission,
};
#[cfg(all(
    test,
    feature = "n384-prep",
    not(feature = "n512-m512-parallel-capture"),
    not(feature = "n256-m512-piecewise-cadv33")
))]
pub(crate) use admission::{admit_geometry, geometry};
#[cfg(all(test, feature = "n512-m512-piecewise-cadv33"))]
pub(crate) use admission::{
    admit, diagnostic_reservations, disk_bound, execution_reservations, geometry, observer_work,
    resources, reservations, Reservations,
};
#[cfg(all(test, feature = "n256-m512-piecewise-cadv33"))]
pub(crate) use admission::{
    admit, diagnostic_reservations, disk_bound, execution_reservations, geometry, resources,
    reservations, work_admission,
};
#[cfg(all(
    test,
    feature = "n384-prep",
    not(feature = "n512-m512-parallel-capture"),
    not(feature = "n256-m512-piecewise-cadv33")
))]
pub(crate) use identity::force_w3_identity;
#[cfg(any(
    feature = "n384-prep",
    feature = "n256-m512-piecewise-cadv33"
))]
pub(crate) use profile::EXTERNAL_STOP;
pub(crate) use profile::{EXECUTION, OVERHEAD, PREFLIGHT_SCHEMA, PROFILE, PROVIDER};
#[cfg(any(
    feature = "n512-m512-temporal-h32",
    feature = "n512-m512-temporal-h16"
))]
pub(crate) use profile::{HOST_PROVENANCE, OBSERVER_STATE_SCHEMA};
#[cfg(all(
    test,
    any(
        feature = "n512-m512-temporal-h32",
        feature = "n512-m512-temporal-h16"
    )
))]
pub(crate) use profile::HISTORY_BYTES;

#[cfg(all(
    test,
    any(
        feature = "n512-m512-temporal-h32",
        feature = "n512-m512-temporal-h16"
    )
))]
mod n512_temporal_tests;
#[cfg(all(
    test,
    feature = "n384-prep",
    not(feature = "n512-m512-parallel-capture"),
    not(feature = "n256-m512-piecewise-cadv33")
))]
mod n384_tests;
#[cfg(all(test, feature = "n512-m512-piecewise-cadv33"))]
mod n512_resource_probe;
#[cfg(all(test, feature = "n256-m512-piecewise-cadv33"))]
mod n256_m512_resource_probe;
