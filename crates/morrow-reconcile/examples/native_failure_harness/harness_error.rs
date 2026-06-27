use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub(crate) enum HarnessError {
    CountMismatch {
        scenario: &'static str,
        field: &'static str,
        expected: usize,
        actual: usize,
    },
    UnexpectedOutcome {
        scenario: &'static str,
        expected: &'static str,
        actual: String,
    },
}

impl fmt::Display for HarnessError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::CountMismatch {
                scenario,
                field,
                expected,
                actual,
            } => write!(
                formatter,
                "{scenario} expected {field}={expected}, got {field}={actual}"
            ),
            Self::UnexpectedOutcome {
                scenario,
                expected,
                actual,
            } => write!(formatter, "{scenario} expected {expected}, got {actual}"),
        }
    }
}

impl Error for HarnessError {}
