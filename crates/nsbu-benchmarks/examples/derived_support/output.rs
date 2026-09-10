//! Streaming scalar bit words preserves tensor entries, derivative order and physical geometry.
use super::{ExportError, Workspace};
use nsbu_benchmarks::smooth_run::ReconstructedRun;
use nsbu_solver::{diagnostics::derivatives::Derivative, domain::Domain};
use std::io::Write;
impl Workspace {
    pub fn write(
        &mut self,
        out: &mut impl Write,
        run: &ReconstructedRun,
        joint_bytes: usize,
        cap_bytes: usize,
    ) -> Result<(), ExportError> {
        self.write_header(out, run, joint_bytes, cap_bytes)?;
        for component in 0..3 {
            self.velocity(out, run, component)?;
        }
        for component in 0..3 {
            let sampled = self
                .scalar
                .sample(&self.curl[component], Derivative::new([0; 3])?)?;
            scalar(out, "vorticity", component, [0; 3], sampled.values, true)?;
        }
        for index in 0..4 {
            let mut orders = [0; 3];
            if index > 0 {
                orders[index - 1] = 1;
            }
            let sampled = self
                .pressure_scalar
                .sample(&self.pressure, Derivative::new(orders)?)?;
            scalar(out, "pressure", 0, orders, sampled.values, index < 3)?;
        }
        writeln!(out, "]}}")?;
        out.flush()?;
        Ok(())
    }
    fn write_header(
        &self,
        out: &mut impl Write,
        run: &ReconstructedRun,
        joint_bytes: usize,
        cap_bytes: usize,
    ) -> Result<(), ExportError> {
        let source = run.state().plan().domain();
        let n = source.layout().dimensions()[0];
        let method = run.history().controller().configuration().method;
        let rhs_calls: usize = run.work().iter().map(|work| work.calls()).sum();
        writeln!(out,"{{\"schema\":\"NSBU_DERIVED_1\",\"grid\":{n},\"samples\":{},\"method\":\"{method:?}\",\"tick_exponent\":-16,\"elapsed_ticks\":128,\"macro_step_ticks\":16,\"macro_steps\":8,\"fine_steps_committed\":16,\"reference_assignments\":0,\"accepted_pde_windows\":0,\"rhs_calls\":{rhs_calls},\"reservation_bytes\":{joint_bytes},\"cap_bytes\":{cap_bytes},\"diagnostic_scalar_transforms\":55,\"state\":[",2*n)?;
        crate::arithmetic_support::field(
            out,
            source,
            [
                run.state().component(0)?,
                run.state().component(1)?,
                run.state().component(2)?,
            ],
        )?;
        writeln!(out, "],\"pressure_force\":[")?;
        crate::arithmetic_support::field(
            out,
            Domain::new([2 * n; 3], [1.0; 3], 1.0)?,
            self.force_values.each_ref().map(Vec::as_slice),
        )?;
        writeln!(out, "],\"fields\":[")?;
        Ok(())
    }
    fn velocity(
        &mut self,
        out: &mut impl Write,
        run: &ReconstructedRun,
        component: usize,
    ) -> Result<(), ExportError> {
        let values = run.state().component(component)?;
        let sampled = self.scalar.sample(values, Derivative::new([0; 3])?)?;
        scalar(out, "velocity", component, [0; 3], sampled.values, true)?;
        for axis in 0..3 {
            let mut orders = [0; 3];
            orders[axis] = 1;
            let sampled = self.scalar.sample(values, Derivative::new(orders)?)?;
            scalar(out, "velocity", component, orders, sampled.values, true)?;
        }
        for first in 0..3 {
            for second in 0..3 {
                let mut orders = [0; 3];
                orders[first] += 1;
                orders[second] += 1;
                let sampled = self.scalar.sample(values, Derivative::new(orders)?)?;
                scalar(out, "velocity", component, orders, sampled.values, true)?;
            }
        }
        Ok(())
    }
}
fn scalar(
    out: &mut impl Write,
    quantity: &str,
    component: usize,
    orders: [u8; 3],
    values: &[f64],
    comma: bool,
) -> Result<(), ExportError> {
    write!(
        out,
        "{{\"quantity\":\"{quantity}\",\"component\":{component},\"orders\":{orders:?},\"bits\":["
    )?;
    for (index, value) in values.iter().enumerate() {
        if index > 0 {
            write!(out, ",")?;
        }
        write!(out, "{}", value.to_bits())?;
    }
    writeln!(out, "]}}{}", if comma { "," } else { "" })?;
    Ok(())
}
