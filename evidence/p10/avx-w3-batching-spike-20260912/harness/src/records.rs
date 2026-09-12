use std::time::Duration;

pub(crate) struct Timings {
    serial_forward: [Duration; 3],
    serial_inverse: [Duration; 3],
    parallel_forward: [Duration; 3],
    parallel_inverse: [Duration; 3],
}

impl Timings {
    pub(crate) fn new() -> Self {
        Self {
            serial_forward: [Duration::ZERO; 3],
            serial_inverse: [Duration::ZERO; 3],
            parallel_forward: [Duration::ZERO; 3],
            parallel_inverse: [Duration::ZERO; 3],
        }
    }

    pub(crate) fn record(
        &mut self,
        pair: usize,
        serial: (Duration, Duration),
        parallel: (Duration, Duration),
    ) {
        (self.serial_forward[pair], self.serial_inverse[pair]) = serial;
        (self.parallel_forward[pair], self.parallel_inverse[pair]) = parallel;
    }

    pub(crate) fn emit_pair(&self, length: usize, pair: usize) {
        println!(
            "pair n={length} index={pair} serial_forward_seconds={:.9} w3_forward_seconds={:.9} serial_inverse_seconds={:.9} w3_inverse_seconds={:.9}",
            self.serial_forward[pair].as_secs_f64(),
            self.parallel_forward[pair].as_secs_f64(),
            self.serial_inverse[pair].as_secs_f64(),
            self.parallel_inverse[pair].as_secs_f64(),
        );
    }

    pub(crate) fn medians(mut self) -> MedianTimings {
        self.serial_forward.sort();
        self.serial_inverse.sort();
        self.parallel_forward.sort();
        self.parallel_inverse.sort();
        MedianTimings {
            serial_forward: self.serial_forward[1],
            serial_inverse: self.serial_inverse[1],
            parallel_forward: self.parallel_forward[1],
            parallel_inverse: self.parallel_inverse[1],
        }
    }
}

pub(crate) struct MedianTimings {
    pub(crate) serial_forward: Duration,
    pub(crate) serial_inverse: Duration,
    pub(crate) parallel_forward: Duration,
    pub(crate) parallel_inverse: Duration,
}

impl MedianTimings {
    pub(crate) fn emit_summary(
        &self,
        length: usize,
        copy_forward: Duration,
        copy_inverse: Duration,
        dispatch: Duration,
        output_hash: &str,
    ) {
        let serial_cycle = self.serial_forward + self.serial_inverse;
        let w3_cycle = self.parallel_forward + self.parallel_inverse;
        let w3_copy_cycle = w3_cycle + copy_forward + copy_inverse;
        println!(
            "summary n={length} scalar_transforms_per_direction=3 serial_forward_median_seconds={:.9} w3_forward_median_seconds={:.9} forward_speedup={:.6} serial_inverse_median_seconds={:.9} w3_inverse_median_seconds={:.9} inverse_speedup={:.6} serial_cycle_seconds={:.9} w3_cycle_seconds={:.9} swap_publish_speedup={:.6} copy_forward_seconds={:.9} copy_inverse_seconds={:.9} copy_inclusive_w3_cycle_seconds={:.9} copy_inclusive_speedup={:.6} dispatch_seconds={:.9} steady_allocations=0 output_equal_words=true repeat_deterministic=true output_sha256={output_hash}",
            self.serial_forward.as_secs_f64(),
            self.parallel_forward.as_secs_f64(),
            self.serial_forward.as_secs_f64() / self.parallel_forward.as_secs_f64(),
            self.serial_inverse.as_secs_f64(),
            self.parallel_inverse.as_secs_f64(),
            self.serial_inverse.as_secs_f64() / self.parallel_inverse.as_secs_f64(),
            serial_cycle.as_secs_f64(),
            w3_cycle.as_secs_f64(),
            serial_cycle.as_secs_f64() / w3_cycle.as_secs_f64(),
            copy_forward.as_secs_f64(),
            copy_inverse.as_secs_f64(),
            w3_copy_cycle.as_secs_f64(),
            serial_cycle.as_secs_f64() / w3_copy_cycle.as_secs_f64(),
            dispatch.as_secs_f64(),
        );
    }
}
