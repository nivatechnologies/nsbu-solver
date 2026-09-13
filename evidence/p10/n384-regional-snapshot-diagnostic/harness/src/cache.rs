use nsbu_benchmarks::fields::reference::ReferenceEvaluation;

pub(crate) const WORDS: usize = 30;
const VELOCITY_START: usize = 0;
const GRADIENT_START: usize = 3;
const HESSIAN_START: usize = 12;

/// Packed analytical velocity, gradient and six symmetric Hessian entries per component.
#[derive(Clone, Copy, Debug)]
#[repr(transparent)]
pub(crate) struct PackedReference([f64; WORDS]);

impl Default for PackedReference {
    fn default() -> Self {
        Self([0.0; WORDS])
    }
}

impl PackedReference {
    pub(crate) fn pack_checked(value: &ReferenceEvaluation) -> Result<Self, String> {
        let mut words = [0.0; WORDS];
        words[VELOCITY_START..GRADIENT_START].copy_from_slice(&value.velocity);
        for component in 0..3 {
            for axis in 0..3 {
                words[gradient_index(component, axis)] = value.gradient[component][axis];
            }
            for (slot, [first, second]) in canonical_pairs().into_iter().enumerate() {
                words[HESSIAN_START + 6 * component + slot] =
                    value.hessian[component][first][second];
            }
        }
        let packed = Self(words);
        for component in 0..3 {
            for first in 0..3 {
                for second in 0..3 {
                    if packed.hessian(component, first, second).to_bits()
                        != value.hessian[component][first][second].to_bits()
                    {
                        return Err(format!(
                            "mixed-partial packing mismatch at component={component},first={first},second={second}"
                        ));
                    }
                }
            }
        }
        Ok(packed)
    }

    pub(crate) fn velocity(self, component: usize) -> f64 {
        self.0[VELOCITY_START + component]
    }

    pub(crate) fn gradient(self, component: usize, axis: usize) -> f64 {
        self.0[gradient_index(component, axis)]
    }

    pub(crate) fn hessian(self, component: usize, first: usize, second: usize) -> f64 {
        self.0[HESSIAN_START + 6 * component + symmetric_slot(first, second)]
    }
}

fn gradient_index(component: usize, axis: usize) -> usize {
    GRADIENT_START + 3 * component + axis
}

const fn canonical_pairs() -> [[usize; 2]; 6] {
    [[0, 0], [0, 1], [0, 2], [1, 1], [1, 2], [2, 2]]
}

fn symmetric_slot(first: usize, second: usize) -> usize {
    match [first.min(second), first.max(second)] {
        [0, 0] => 0,
        [0, 1] => 1,
        [0, 2] => 2,
        [1, 1] => 3,
        [1, 2] => 4,
        [2, 2] => 5,
        _ => unreachable!("three-dimensional derivative axis"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use nsbu_benchmarks::fields::reference;
    use nsbu_benchmarks::time::BenchmarkTime;
    use nsbu_solver::domain::TickClock;

    #[test]
    fn packed_layout_is_exactly_thirty_binary64_words() {
        assert_eq!(size_of::<PackedReference>(), WORDS * size_of::<f64>());
    }

    #[test]
    fn canonical_mixed_partials_equal_all_independent_ordered_entries() {
        let clock = TickClock::restore(-20, 8192, 512, 7680).unwrap();
        let time = BenchmarkTime::new(clock).unwrap();
        for point in [
            [0.0, 0.0, 0.0],
            [0.01, 0.02, 0.03],
            [0.2, 0.0, 0.0],
            [0.305, 0.0, 0.0],
            [0.35, 0.0, 0.0],
        ] {
            let independent = reference::evaluate(point, time).unwrap();
            let packed = PackedReference::pack_checked(&independent).unwrap();
            for component in 0..3 {
                for first in 0..3 {
                    for second in 0..3 {
                        assert_eq!(
                            packed.hessian(component, first, second).to_bits(),
                            independent.hessian[component][first][second].to_bits()
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn asymmetric_ordered_input_is_refused() {
        let clock = TickClock::restore(-20, 8192, 512, 7680).unwrap();
        let time = BenchmarkTime::new(clock).unwrap();
        let mut value = reference::evaluate([0.01, 0.02, 0.03], time).unwrap();
        value.hessian[0][1][0] += 1.0;
        assert!(PackedReference::pack_checked(&value)
            .unwrap_err()
            .contains("mixed-partial"));
    }
}
