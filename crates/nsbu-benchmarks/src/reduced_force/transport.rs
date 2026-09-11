//! Cartesian momentum terms from the exact identities u=(x*C-y*B,y*C+x*B,D).
//! C=-A_z and D=2*A+2*w*A_w are regular at the axis; no division by radius occurs.
use super::jet::Jet;
use crate::BenchmarkError;
type MomentumTerms = [[f64; 4]; 3];
pub(super) fn evaluate(
    xy: [f64; 2],
    w: Jet,
    a: Jet,
    b: Jet,
    p: Jet,
) -> Result<([f64; 3], MomentumTerms), BenchmarkError> {
    let c = a.derivative(1)?.scale(-1.0)?;
    let d = a.scale(2.0)?.plus(w.times(a.derivative(0)?)?.scale(2.0)?)?;
    let [temporal, advective, viscous] = transport([c, b, d], w.value())?;
    let pressure = [
        2.0 * p.derivative(0)?.value(),
        0.0,
        p.derivative(1)?.value(),
    ];
    let terms = [temporal, advective, viscous, pressure].map(|terms| cartesian(xy, terms));
    let velocity = cartesian(xy, [c.value(), b.value(), d.value()]);
    Ok((
        velocity,
        std::array::from_fn(|i| std::array::from_fn(|j| terms[j][i])),
    ))
}
fn transport(jets: [Jet; 3], radius: f64) -> Result<[[f64; 3]; 3], BenchmarkError> {
    let [c, b, d] = jets;
    let [cv, bv, dv] = jets.map(Jet::value);
    let [cw, bw, dw] = derivatives(jets, 0)?;
    let [cz, bz, dz] = derivatives(jets, 1)?;
    let advective = [
        cv * cv - bv * bv + 2.0 * radius * cv * cw + dv * cz,
        2.0 * cv * bv + 2.0 * radius * cv * bw + dv * bz,
        2.0 * radius * cv * dw + dv * dz,
    ];
    let viscous = [
        -laplacian(c, radius, 8.0)?,
        -laplacian(b, radius, 8.0)?,
        -laplacian(d, radius, 4.0)?,
    ];
    Ok([derivatives(jets, 2)?, advective, viscous])
}
fn derivatives(jets: [Jet; 3], axis: usize) -> Result<[f64; 3], BenchmarkError> {
    Ok([
        jets[0].derivative(axis)?.value(),
        jets[1].derivative(axis)?.value(),
        jets[2].derivative(axis)?.value(),
    ])
}
fn laplacian(value: Jet, w: f64, linear: f64) -> Result<f64, BenchmarkError> {
    let radial = value.derivative(0)?;
    Ok(linear * radial.value()
        + 4.0 * w * radial.derivative(0)?.value()
        + value.derivative(1)?.derivative(1)?.value())
}
fn cartesian([x, y]: [f64; 2], [radial, swirl, axial]: [f64; 3]) -> [f64; 3] {
    [x * radial - y * swirl, y * radial + x * swirl, axial]
}
