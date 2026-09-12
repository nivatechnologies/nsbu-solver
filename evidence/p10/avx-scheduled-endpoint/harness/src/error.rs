use nsbu_solver::SolverError;
use std::{error::Error, fmt, io};

pub type AnyResult<T> = Result<T, HarnessError>;

#[derive(Debug)]
pub enum HarnessError {
    Numerical(SolverError),
    Io(io::Error),
    Rejected { ticks: u128, ratios: [f64; 2] },
}

impl fmt::Display for HarnessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Numerical(error) => write!(formatter, "numerical:{error:?}"),
            Self::Io(error) => write!(formatter, "io:{error}"),
            Self::Rejected { ticks, ratios } => {
                write!(
                    formatter,
                    "attempt_rejected:ticks={ticks}:ratios={ratios:?}"
                )
            }
        }
    }
}

impl Error for HarnessError {}

impl From<SolverError> for HarnessError {
    fn from(error: SolverError) -> Self {
        Self::Numerical(error)
    }
}

impl From<io::Error> for HarnessError {
    fn from(error: io::Error) -> Self {
        Self::Io(error)
    }
}
