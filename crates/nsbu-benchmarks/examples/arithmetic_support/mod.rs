//! Shared streaming representation for independent-arithmetic data producers.
pub mod evolution;
use nsbu_solver::{domain::Domain, integrators::method::Method, Complex64, SolverError};
use std::io::{self, Write};

pub const CAP: usize = 64 * 1024 * 1024;

#[derive(Debug)]
pub enum ExportError {
    Numerical(SolverError),
    Io(io::Error),
}
impl From<SolverError> for ExportError {
    fn from(value: SolverError) -> Self {
        Self::Numerical(value)
    }
}
impl From<io::Error> for ExportError {
    fn from(value: io::Error) -> Self {
        Self::Io(value)
    }
}
impl std::fmt::Display for ExportError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Numerical(error) => write!(f, "numerical export refused: {error:?}"),
            Self::Io(error) => write!(f, "export write failed: {error}"),
        }
    }
}
impl std::error::Error for ExportError {}

pub fn grid(value: Option<String>) -> Result<usize, SolverError> {
    let n = value
        .ok_or(SolverError::InvalidPayload)?
        .parse()
        .map_err(|_| SolverError::InvalidDomain)?;
    if !matches!(n, 4 | 8 | 12) {
        return Err(SolverError::InvalidDomain);
    }
    Ok(n)
}
pub fn method(value: Option<String>) -> Result<Method, SolverError> {
    match value.as_deref() {
        Some("CM") => Ok(Method::CoxMatthews),
        Some("HO") => Ok(Method::HochbruckOstermann),
        _ => Err(SolverError::InvalidPayload),
    }
}

// No serialization array is allocated. The caller owns the writer and its buffering policy.
pub fn field(
    out: &mut impl Write,
    domain: Domain,
    values: [&[Complex64]; 3],
) -> Result<(), ExportError> {
    let layout = domain.layout();
    if values.iter().any(|v| v.len() != layout.half_len()) {
        return Err(SolverError::InvalidPayload.into());
    }
    for index in 0..layout.half_len() {
        let mode = layout.mode(layout.position(index)?)?;
        let [a, b, c] = values.map(|v| v[index]);
        let separator = if index + 1 == layout.half_len() {
            ""
        } else {
            ","
        };
        writeln!(
            out,
            "{{\"mode\":{mode:?},\"bits\":[[{},{}],[{},{}],[{},{}]]}}{separator}",
            a.re.to_bits(),
            a.im.to_bits(),
            b.re.to_bits(),
            b.im.to_bits(),
            c.re.to_bits(),
            c.im.to_bits()
        )?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn argument_contract_refuses_unsupported_profiles() {
        for value in [None, Some("3"), Some("invalid")] {
            assert!(grid(value.map(str::to_owned)).is_err());
        }
        assert_eq!(grid(Some("12".into())), Ok(12));
        assert_eq!(method(Some("CM".into())), Ok(Method::CoxMatthews));
        assert_eq!(method(Some("HO".into())), Ok(Method::HochbruckOstermann));
        assert!(method(None).is_err());
        assert!(method(Some("RK4".into())).is_err());
    }

    #[test]
    fn shape_refusal_happens_before_writing() {
        let domain = Domain::new([4; 3], [1.0; 3], 1.0).unwrap();
        let mut bytes = Vec::new();
        assert!(field(&mut bytes, domain, [&[]; 3]).is_err());
        assert!(bytes.is_empty());
        assert!(!ExportError::Numerical(SolverError::InvalidPayload)
            .to_string()
            .is_empty());
        assert!(!ExportError::Io(io::Error::other("closed"))
            .to_string()
            .is_empty());
    }
}
