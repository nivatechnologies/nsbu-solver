//! Mutually exclusive profile admission, the exact compile-time profile
//! constants and their schema strings.
use crate::{artifact, cache::CachedReducedForce, schedule, timed_rhs::TimedRhs};
use nsbu_solver::integrators::rhs::SpectralRhs;

#[cfg(all(feature = "n256", feature = "n384-prep"))]
compile_error!("n256 and n384-prep are mutually exclusive profiles");
#[cfg(all(feature = "n384-h32", feature = "n384-h64"))]
compile_error!("n384-h32 and n384-h64 are mutually exclusive profiles");
#[cfg(all(
    feature = "n512-m512-piecewise-cadv33",
    any(
        feature = "n384-h32",
        feature = "n384-h64",
        feature = "n384-piecewise",
        feature = "n384-piecewise-cadv33",
        feature = "n384-m512-piecewise-cadv33"
    )
))]
compile_error!("n512-m512-piecewise-cadv33 is mutually exclusive with every n384 profile");
#[cfg(all(
    feature = "n512-m512-piecewise-cadv33",
    any(
        feature = "n512-m512-temporal-h32",
        feature = "n512-m512-temporal-h16"
    )
))]
compile_error!("n512-m512-piecewise-cadv33 is mutually exclusive with every temporal N512 profile");
#[cfg(all(
    feature = "n512-m512-temporal-h32",
    any(
        feature = "n512-m512-temporal-h16",
        feature = "n256",
        feature = "n256-m512-piecewise-cadv33",
        feature = "n384-h32",
        feature = "n384-h64",
        feature = "n384-piecewise",
        feature = "n384-piecewise-cadv33",
        feature = "n384-m512-piecewise-cadv33"
    )
))]
compile_error!("n512-m512-temporal-h32 is a mutually exclusive N512 temporal profile");
#[cfg(all(
    feature = "n512-m512-temporal-h16",
    any(
        feature = "n512-m512-temporal-h32",
        feature = "n256",
        feature = "n256-m512-piecewise-cadv33",
        feature = "n384-h32",
        feature = "n384-h64",
        feature = "n384-piecewise",
        feature = "n384-piecewise-cadv33",
        feature = "n384-m512-piecewise-cadv33"
    )
))]
compile_error!("n512-m512-temporal-h16 is a mutually exclusive N512 temporal profile");
#[cfg(all(
    feature = "n256-m512-piecewise-cadv33",
    any(
        feature = "n256",
        feature = "n512-m512-piecewise-cadv33",
        feature = "n512-m512-temporal-h32",
        feature = "n512-m512-temporal-h16",
        feature = "n384-h32",
        feature = "n384-h64",
        feature = "n384-piecewise",
        feature = "n384-piecewise-cadv33",
        feature = "n384-m512-piecewise-cadv33"
    )
))]
compile_error!("n256-m512-piecewise-cadv33 is a mutually exclusive profile");
#[cfg(any(
    all(feature = "n384-h32", feature = "n384-piecewise"),
    all(feature = "n384-h64", feature = "n384-piecewise"),
    all(feature = "n384-h32", feature = "n384-piecewise-cadv33"),
    all(feature = "n384-h64", feature = "n384-piecewise-cadv33"),
    all(feature = "n384-piecewise", feature = "n384-piecewise-cadv33"),
    all(feature = "n384-h32", feature = "n384-m512-piecewise-cadv33"),
    all(feature = "n384-h64", feature = "n384-m512-piecewise-cadv33"),
    all(feature = "n384-piecewise", feature = "n384-m512-piecewise-cadv33"),
    all(
        feature = "n384-piecewise-cadv33",
        feature = "n384-m512-piecewise-cadv33"
    )
))]
compile_error!("select only one exact n384 profile");
#[cfg(all(
    feature = "n384-prep",
    not(any(
        feature = "n384-h32",
        feature = "n384-h64",
        feature = "n384-piecewise",
        feature = "n384-piecewise-cadv33",
        feature = "n384-m512-piecewise-cadv33",
        feature = "n512-m512-piecewise-cadv33",
        feature = "n512-m512-temporal-h32",
        feature = "n512-m512-temporal-h16",
        feature = "n256-m512-piecewise-cadv33"
    ))
))]
compile_error!("select an exact top-level n384 feature");

#[cfg(not(any(feature = "n256", feature = "n384-prep")))]
pub const N: usize = 192;
#[cfg(all(feature = "n256", not(feature = "n384-prep")))]
pub const N: usize = 256;
#[cfg(all(
    feature = "n384-prep",
    not(feature = "n256"),
    not(feature = "n512-m512-parallel-capture"),
    not(feature = "n256-m512-piecewise-cadv33")
))]
pub const N: usize = 384;
#[cfg(feature = "n512-m512-parallel-capture")]
pub const N: usize = 512;
#[cfg(feature = "n256-m512-piecewise-cadv33")]
pub const N: usize = 256;
#[cfg(not(any(
    feature = "n384-m512-piecewise-cadv33",
    feature = "n512-m512-parallel-capture",
    feature = "n256-m512-piecewise-cadv33"
)))]
pub const M: usize = 384;
#[cfg(any(
    feature = "n384-m512-piecewise-cadv33",
    feature = "n512-m512-parallel-capture",
    feature = "n256-m512-piecewise-cadv33"
))]
pub const M: usize = 512;
#[cfg(not(any(
    feature = "n512-m512-parallel-capture",
    feature = "n256-m512-piecewise-cadv33"
)))]
pub const OBSERVER_M: usize = 768;
#[cfg(feature = "n512-m512-parallel-capture")]
pub const OBSERVER_M: usize = 1024;
#[cfg(feature = "n256-m512-piecewise-cadv33")]
pub const OBSERVER_M: usize = 512;
pub const WORKERS: usize = 32;
#[cfg(not(feature = "n384-prep"))]
pub const CAP: usize = 103_079_215_104;
#[cfg(all(
    feature = "n384-prep",
    not(feature = "n512-m512-parallel-capture"),
    not(feature = "n256-m512-piecewise-cadv33")
))]
pub const CAP: usize = 192 * 1024 * 1024 * 1024;
#[cfg(feature = "n512-m512-piecewise-cadv33")]
pub const CAP: usize = 207_627_451_152;
#[cfg(feature = "n512-m512-temporal-h32")]
pub const CAP: usize = 207_627_647_760;
#[cfg(feature = "n512-m512-temporal-h16")]
pub const CAP: usize = 207_628_040_976;
#[cfg(feature = "n256-m512-piecewise-cadv33")]
pub const CAP: usize = 68_719_476_736;
#[cfg(feature = "n512-m512-parallel-capture")]
pub const FFT_WORKERS: usize = 8;
#[cfg(not(feature = "n384-prep"))]
pub const ADVECTIVE_LIMIT: f64 = 0.45;
#[cfg(all(
    feature = "n384-prep",
    not(feature = "n384-piecewise-common"),
    not(feature = "n512-m512-parallel-capture")
))]
pub const ADVECTIVE_LIMIT: f64 = 0.8;
#[cfg(feature = "n384-piecewise")]
pub const ADVECTIVE_LIMIT: f64 = 1.6;
#[cfg(any(
    feature = "n384-piecewise-cadv33",
    feature = "n384-m512-piecewise-cadv33",
    feature = "n512-m512-parallel-capture",
    feature = "n256-m512-piecewise-cadv33"
))]
pub const ADVECTIVE_LIMIT: f64 = 3.3;
pub(crate) const HISTORY_BYTES: usize = schedule::MAXIMUM_ATTEMPTS * 4096;
const TIMER_OVERHEAD: usize = TimedRhs::<SpectralRhs<CachedReducedForce>>::reservation_overhead();
pub(crate) const OVERHEAD: usize = artifact::BUFFER_BYTES + HISTORY_BYTES + TIMER_OVERHEAD + 64 * 1024;

#[cfg(not(any(feature = "n256", feature = "n384-prep")))]
pub(crate) const PROFILE: &str = "n192-m384";
#[cfg(all(feature = "n256", not(feature = "n384-prep")))]
pub(crate) const PROFILE: &str = "n256-m384";
#[cfg(all(feature = "n384-h32", not(feature = "n256")))]
pub(crate) const PROFILE: &str = "n384-m384-h32-cadv08-w3-f13c29c";
#[cfg(all(feature = "n384-h64", not(feature = "n256")))]
pub(crate) const PROFILE: &str = "n384-m384-h64-cadv08-w3-f13c29c";
#[cfg(all(feature = "n384-piecewise", not(feature = "n256")))]
pub(crate) const PROFILE: &str = "n384-m384-h64to2048-h128to4096-cadv16-w3-f13c29c";
#[cfg(all(feature = "n384-piecewise-cadv33", not(feature = "n256")))]
pub(crate) const PROFILE: &str = "n384-m384-h64to2048-h128to4096-cadv33-w3-f13c29c";
#[cfg(feature = "n384-m512-piecewise-cadv33")]
pub(crate) const PROFILE: &str = "n384-m512-h64to2048-h128to4096-cadv33-w3-f13c29c";
#[cfg(feature = "n512-m512-piecewise-cadv33")]
pub(crate) const PROFILE: &str = "n512-m512-h64to2048-h128to4096-cadv33-w3-pfft1ed6995";
#[cfg(feature = "n512-m512-temporal-h32")]
pub(crate) const PROFILE: &str = "n512-m512-h32to2048-h64to4096-cadv33-w3-pfft1ed6995";
#[cfg(feature = "n512-m512-temporal-h16")]
pub(crate) const PROFILE: &str = "n512-m512-h16to2048-h32to4096-cadv33-w3-pfft1ed6995";
#[cfg(feature = "n256-m512-piecewise-cadv33")]
pub(crate) const PROFILE: &str = "n256-m512-h64to2048-h128to4096-cadv33-w3-f13c29c";
#[cfg(all(
    feature = "n384-prep",
    not(feature = "n512-m512-parallel-capture"),
    not(feature = "n256-m512-piecewise-cadv33")
))]
pub(crate) const PREFLIGHT_SCHEMA: &str = "p10-avx-n384-preflight-v1";
#[cfg(feature = "n512-m512-piecewise-cadv33")]
pub(crate) const PREFLIGHT_SCHEMA: &str = "p10-avx-n512-m512-endpoint-capture-preflight-v1";
#[cfg(feature = "n512-m512-temporal-h32")]
pub(crate) const PREFLIGHT_SCHEMA: &str = "p10-avx-n512-m512-temporal-h32-capture-preflight-v1";
#[cfg(feature = "n512-m512-temporal-h16")]
pub(crate) const PREFLIGHT_SCHEMA: &str = "p10-avx-n512-m512-temporal-h16-capture-preflight-v1";
#[cfg(feature = "n256-m512-piecewise-cadv33")]
pub(crate) const PREFLIGHT_SCHEMA: &str = "p10-avx-n256-m512-endpoint-capture-preflight-v1";
#[cfg(not(feature = "n384-prep"))]
pub(crate) const PREFLIGHT_SCHEMA: &str = "p10-avx-scheduled-endpoint-v2";
#[cfg(all(
    feature = "n384-prep",
    not(feature = "n512-m512-parallel-capture"),
    not(feature = "n256-m512-piecewise-cadv33")
))]
pub(crate) const EXECUTION: &str = "separate-rhs-force-w3";
#[cfg(feature = "n512-m512-parallel-capture")]
pub(crate) const EXECUTION: &str = "separate-rhs-force-w3-parallel8";
#[cfg(feature = "n256-m512-piecewise-cadv33")]
pub(crate) const EXECUTION: &str = "separate-rhs-force-w3-offline-capture";
#[cfg(not(feature = "n384-prep"))]
pub(crate) const EXECUTION: &str = "serial-component-fft";
#[cfg(all(feature = "n384-prep", not(feature = "n512-m512-parallel-capture")))]
pub(crate) const PROVIDER: &str = "parallel-reduced-v2-force-w3-attempt-cache";
#[cfg(feature = "n512-m512-parallel-capture")]
pub(crate) const PROVIDER: &str = "parallel-reduced-v2-force-w3-parallel8-attempt-cache";
#[cfg(not(feature = "n384-prep"))]
pub(crate) const PROVIDER: &str = "parallel-reduced-attempt-cache";
#[cfg(all(
    feature = "n384-prep",
    not(feature = "n384-piecewise-common"),
    not(feature = "n512-m512-parallel-capture")
))]
pub(crate) const EXTERNAL_STOP: &str = "pgid-watchdog-v1-starttime-cmdline-deadline";
#[cfg(all(
    feature = "n384-piecewise-common",
    not(feature = "n512-m512-parallel-capture"),
    not(feature = "n256-m512-piecewise-cadv33")
))]
pub(crate) const EXTERNAL_STOP: &str = "pgid-watchdog-v2-starttime-cmdline-deadline";
#[cfg(any(
    feature = "n512-m512-parallel-capture",
    feature = "n256-m512-piecewise-cadv33"
))]
pub(crate) const EXTERNAL_STOP: &str = "pgid-watchdog-v3-confirmed-identity-absolute-deadline";
#[cfg(any(
    feature = "n512-m512-temporal-h32",
    feature = "n512-m512-temporal-h16"
))]
pub(crate) const HOST_PROVENANCE: &str = "explicit-reviewed-host-required";
#[cfg(feature = "n512-m512-temporal-h32")]
pub(crate) const OBSERVER_STATE_SCHEMA: &str = "p10-avx-n512-m512-h32-observer-state-v1";
#[cfg(feature = "n512-m512-temporal-h16")]
pub(crate) const OBSERVER_STATE_SCHEMA: &str = "p10-avx-n512-m512-h16-observer-state-v1";
