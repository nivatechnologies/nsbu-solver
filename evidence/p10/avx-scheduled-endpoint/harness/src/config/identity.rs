//! The exact profile identity string and its W3 layout witnesses.
use super::*;
#[cfg(feature = "n384-prep")]
use crate::artifact;
use crate::schedule;
use nsbu_benchmarks::CASE_SHA256;

pub fn identity() -> String {
    #[cfg(feature = "n512-m512-piecewise-cadv33")]
    return format!(
        "source={};case={CASE_SHA256};profile={PROFILE};backend=rustfft-6.4.1-avx-avx2-fma;library_source=1ed699568be70dedf72492324be08600d4407c02;prototype_source=b09fb7719c66cfddb04e56a37fe3f0d0fadba5a5;provider=parallel-reduced-v2-force-w3-parallel8-attempt-cache;rhs_w3=layout768-width3-bidirectional-add21812652048;force_w3={};rhs_timer={};retained={N};force_samples={M};observer_force_samples={OBSERVER_M};observer_conservative={};observer_execution=offline-baccus-required;sampling_workers={WORKERS};rhs_w3_persistent_callers=3;rhs_fft_helpers={FFT_WORKERS};rhs_fft_total_workers=11;provider_w3_persistent_callers=3;provider_fft_helpers={FFT_WORKERS};provider_fft_total_workers=11;method=cox-matthews;schedule={};endpoint={};advective_limit={ADVECTIVE_LIMIT};execution_cap={CAP};artifact_cap={};schema=p10-avx-n512-observer-state-v1;attempt_schema=p10-avx-scheduled-attempt-v3;resume=unsupported;host=sulaco;numa=whole-host-unbound-all-visible-cpus-memory;external_stop={EXTERNAL_STOP}",
        env!("RUN_SOURCE"),
        force_w3_identity(),
        crate::timed_rhs::IDENTITY,
        2 * N,
        schedule::IDENTITY,
        schedule::ENDPOINT,
        artifact::DISK_CAP_BYTES,
    );
    #[cfg(all(
        feature = "n512-m512-parallel-capture",
        not(feature = "n512-m512-piecewise-cadv33")
    ))]
    return format!(
        "source={};case={CASE_SHA256};profile={PROFILE};backend=rustfft-6.4.1-avx-avx2-fma;library_source=1ed699568be70dedf72492324be08600d4407c02;prototype_source=b09fb7719c66cfddb04e56a37fe3f0d0fadba5a5;provider=parallel-reduced-v2-force-w3-parallel8-attempt-cache;rhs_w3=layout768-width3-bidirectional-add21812652048;force_w3={};rhs_timer={};retained={N};force_samples={M};observer_force_samples={OBSERVER_M};observer_conservative={};observer_execution=offline-baccus-required;host_provenance={HOST_PROVENANCE};sampling_workers={WORKERS};rhs_w3_persistent_callers=3;rhs_fft_helpers={FFT_WORKERS};rhs_fft_total_workers=11;provider_w3_persistent_callers=3;provider_fft_helpers={FFT_WORKERS};provider_fft_total_workers=11;method=cox-matthews;schedule={};endpoint={};maximum_attempts={};advective_limit={ADVECTIVE_LIMIT};execution_cap={CAP};artifact_cap={};schema={OBSERVER_STATE_SCHEMA};attempt_schema=p10-avx-scheduled-attempt-v3;resume=unsupported;numa=whole-host-unbound-all-visible-cpus-memory;external_stop={EXTERNAL_STOP}",
        env!("RUN_SOURCE"),
        force_w3_identity(),
        crate::timed_rhs::IDENTITY,
        2 * N,
        schedule::IDENTITY,
        schedule::ENDPOINT,
        schedule::MAXIMUM_ATTEMPTS,
        artifact::DISK_CAP_BYTES,
    );
    #[cfg(all(
        feature = "n384-prep",
        not(feature = "n512-m512-parallel-capture"),
        not(feature = "n256-m512-piecewise-cadv33")
    ))]
    return format!(
        "source={};case={CASE_SHA256};profile={PROFILE};backend=rustfft-6.4.1-avx-avx2-fma;w3_source=f13c29c9ae91d0b8cf7a790132deb9bd076911c0;provider=parallel-reduced-v2-force-w3-attempt-cache;rhs_w3=layout576-width3-bidirectional-add9200926592;force_w3={};rhs_timer={};retained={N};force_samples={M};observer_force_samples={OBSERVER_M};observer_conservative={};sampling_workers={WORKERS};rhs_w3_workers=3;provider_w3_workers=3;method=cox-matthews;schedule={};endpoint={};advective_limit={ADVECTIVE_LIMIT};execution_cap={CAP};artifact_cap={};schema=p10-avx-n384-every-step-v1;attempt_schema=p10-avx-scheduled-attempt-v3;resume=unsupported;host=sulaco;numa=whole-host-unbound-all-visible-cpus-memory;external_stop={EXTERNAL_STOP}",
        env!("RUN_SOURCE"),
        force_w3_identity(),
        crate::timed_rhs::IDENTITY,
        2 * N,
        schedule::IDENTITY,
        schedule::ENDPOINT,
        artifact::DISK_CAP_BYTES,
    );
    #[cfg(feature = "n256-m512-piecewise-cadv33")]
    return format!(
        "source={};case={CASE_SHA256};profile={PROFILE};backend=rustfft-6.4.1-avx-avx2-fma;w3_source=f13c29c9ae91d0b8cf7a790132deb9bd076911c0;provider=parallel-reduced-v2-force-w3-attempt-cache;rhs_w3=layout384-width3-bidirectional-add2734010240;force_w3={};rhs_timer={};retained={N};force_samples={M};observer_force_samples={OBSERVER_M};observer_conservative={};observer_execution=offline-baccus-required;rhs_dealias=384;sampling_workers={WORKERS};rhs_w3_workers=3;provider_w3_workers=3;method=cox-matthews;schedule={};endpoint={};advective_limit={ADVECTIVE_LIMIT};execution_cap={CAP};artifact_cap={};schema=p10-avx-n256-m512-observer-state-v1;attempt_schema=p10-avx-scheduled-attempt-v3;resume=unsupported;host=baccus;numa=whole-host-unbound-all-visible-cpus-memory;external_stop={EXTERNAL_STOP}",
        env!("RUN_SOURCE"),
        force_w3_identity(),
        crate::timed_rhs::IDENTITY,
        2 * N,
        schedule::IDENTITY,
        schedule::ENDPOINT,
        artifact::DISK_CAP_BYTES,
    );
    #[cfg(not(feature = "n384-prep"))]
    format!(
        "source={};case={CASE_SHA256};profile={PROFILE};backend=rustfft-6.4.1-avx-avx2-fma;provider=parallel-reduced-attempt-cache;rhs_timer={};n={N};m={M};workers={WORKERS};method=cox-matthews;step={};endpoint={};advective_limit={ADVECTIVE_LIMIT};cap={CAP};schema=p10-avx-scheduled-endpoint-v2;attempt_schema=p10-avx-scheduled-attempt-v3;resume=unsupported",
        env!("RUN_SOURCE"),
        crate::timed_rhs::IDENTITY,
        schedule::STEP,
        schedule::ENDPOINT,
    )
}

#[cfg(all(
    feature = "n384-prep",
    not(feature = "n384-m512-piecewise-cadv33"),
    not(feature = "n512-m512-parallel-capture"),
    not(feature = "n256-m512-piecewise-cadv33")
))]
pub(crate) fn force_w3_identity() -> &'static str {
    "layout384-width3-forward-add1828040448"
}

#[cfg(any(
    feature = "n384-m512-piecewise-cadv33",
    feature = "n256-m512-piecewise-cadv33"
))]
pub(crate) fn force_w3_identity() -> &'static str {
    "layout512-width3-forward-add4318465792"
}

#[cfg(feature = "n512-m512-parallel-capture")]
pub(crate) fn force_w3_identity() -> &'static str {
    "layout512-width3-forward-add4343035792"
}
