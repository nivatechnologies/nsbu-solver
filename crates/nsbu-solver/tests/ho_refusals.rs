//! Admission and arithmetic refusals for the independently owned HO workspace.
use nsbu_solver::{
    domain::TickClock,
    integrators::{ho_coefficients::HoCoefficients, ho_kernel::HoWorkspace, kernel::RightHandSide},
    Complex64, SolverError,
};

struct Fill(Complex64);
impl RightHandSide for Fill {
    fn evaluate(
        &mut self,
        _state: [&[Complex64]; 3],
        _time: TickClock,
        output: [&mut [Complex64]; 3],
    ) -> Result<(), SolverError> {
        for component in output {
            component.fill(self.0);
        }
        Ok(())
    }
}

#[test]
fn complete_six_vector_reservation_is_required_before_allocation() {
    assert_eq!(
        HoWorkspace::reservation(0),
        Err(SolverError::InvalidPayload)
    );
    assert_eq!(
        HoWorkspace::reservation(usize::MAX),
        Err(SolverError::SizeOverflow)
    );
    let size = HoWorkspace::reservation(3).unwrap();
    assert_eq!(
        size,
        18 * 3 * std::mem::size_of::<Complex64>() + std::mem::size_of::<HoWorkspace>()
    );
    assert!(HoWorkspace::new(3, size - 1).is_err());
    assert!(HoWorkspace::new(3, size).is_ok());
}

#[test]
fn invalid_shapes_steps_and_nonfinite_states_are_refused() {
    let clock = TickClock::from_rest(-6, 64).unwrap();
    let mut work = HoWorkspace::new(1, 4096).unwrap();
    let zero = [Complex64::new(0.0, 0.0)];
    let tables = [HoCoefficients::new(0.0).unwrap()];
    for (dt, ni, nc, no) in [
        (0.0, 1, 1, 1),
        (-1.0, 1, 1, 1),
        (f64::NAN, 1, 1, 1),
        (1.0, 0, 1, 1),
        (1.0, 1, 0, 1),
        (1.0, 1, 1, 0),
    ] {
        let mut output = [zero; 3];
        let [a, b, c] = &mut output;
        let result = work.step(
            [&zero[..ni]; 3],
            [clock; 3],
            dt,
            &tables[..nc],
            &mut Fill(zero[0]),
            [&mut a[..no], b, c],
        );
        assert_eq!(result, Err(SolverError::InvalidPayload));
    }
    for value in [
        Complex64::new(f64::NAN, 0.0),
        Complex64::new(0.0, f64::INFINITY),
    ] {
        let input = [value];
        let mut output = [zero; 3];
        let [a, b, c] = &mut output;
        let result = work.step(
            [&input; 3],
            [clock; 3],
            0.125,
            &tables,
            &mut Fill(zero[0]),
            [a, b, c],
        );
        assert_eq!(result, Err(SolverError::InvalidSpectrum));
    }
}

#[test]
fn invalid_source_and_finite_stage_or_output_overflows_are_refused() {
    let clock = TickClock::from_rest(0, 16).unwrap();
    let zero = [Complex64::new(0.0, 0.0)];
    let mut work = HoWorkspace::new(1, 4096).unwrap();
    for (source, dt, weight) in [
        (f64::INFINITY, 0.125, 1.0),
        (f64::MAX, 4.0, 1.0),
        (1.0, 0.125, f64::INFINITY),
    ] {
        let mut table = HoCoefficients::new(0.0).unwrap();
        table.weights[0] = weight;
        let mut output = [zero; 3];
        let [a, b, c] = &mut output;
        let result = work.step(
            [&zero; 3],
            [clock; 3],
            dt,
            &[table],
            &mut Fill(Complex64::new(source, 0.0)),
            [a, b, c],
        );
        assert_eq!(result, Err(SolverError::InvalidSpectrum));
    }
}
