use std::error::Error;
use std::fmt;

/// Native failure harness validation error.
#[derive(Debug)]
pub enum HarnessError {
    /// A scenario count did not match the expected value.
    CountMismatch {
        /// Scenario identifier.
        scenario: &'static str,
        /// Count field name.
        field: &'static str,
        /// Expected count.
        expected: usize,
        /// Actual count.
        actual: usize,
    },
    /// A scenario emitted an unexpected status or reason.
    UnexpectedOutcome {
        /// Scenario identifier.
        scenario: &'static str,
        /// Expected status or reason.
        expected: &'static str,
        /// Actual status or reason.
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
